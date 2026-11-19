//! The plugins that run: one process each, spoken to over its standard
//! input and output. What they say comes to the app as messages, through
//! [`events`].

use std::collections::HashMap;
use std::io::BufReader;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use ardoise_plugin::protocol::{self, methods, InitializeParams, Message as Rpc, RpcError};
use serde_json::Value;

use super::store::Installed;

/// What a plugin said, or that it stopped.
#[derive(Debug, Clone)]
pub enum Incoming {
    Rpc(String, Rpc),
    /// The plugin's process ended without being asked to: why.
    Crashed(String, String),
}

type Waiter = tokio::sync::oneshot::Sender<Result<Value, RpcError>>;

struct Process {
    stdin: Mutex<Option<ChildStdin>>,
    /// The requests of the editor waiting for their answer.
    pending: Mutex<HashMap<i64, Waiter>>,
    child: Mutex<Child>,
    next_id: AtomicI64,
    /// It is being stopped on purpose: its end is no crash.
    stopping: AtomicBool,
    /// Tells this run of the plugin from the next one.
    run: u64,
}

impl Process {
    fn send(&self, message: &Rpc) -> bool {
        let Ok(mut stdin) = self.stdin.lock() else {
            return false;
        };
        match stdin.as_mut() {
            Some(pipe) => protocol::write_message(pipe, message).is_ok(),
            None => false,
        }
    }
}

fn processes() -> &'static Mutex<HashMap<String, Arc<Process>>> {
    static PROCESSES: OnceLock<Mutex<HashMap<String, Arc<Process>>>> = OnceLock::new();
    PROCESSES.get_or_init(Mutex::default)
}

type Channel = (
    tokio::sync::mpsc::UnboundedSender<Incoming>,
    Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<Incoming>>>,
);

fn channel() -> &'static Channel {
    static CHANNEL: OnceLock<Channel> = OnceLock::new();
    CHANNEL.get_or_init(|| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (tx, Mutex::new(Some(rx)))
    })
}

fn emit(incoming: Incoming) {
    let _ = channel().0.send(incoming);
}

/// What the plugins say, as it comes.
pub fn events() -> iced::Subscription<crate::app::Message> {
    iced::Subscription::run(|| {
        iced::stream::channel(64, async |mut output| {
            use iced::futures::SinkExt;
            let taken = channel().1.lock().ok().and_then(|mut rx| rx.take());
            let Some(mut rx) = taken else {
                // Someone else listens already.
                std::future::pending::<()>().await;
                return;
            };
            while let Some(incoming) = rx.recv().await {
                let message = crate::app::Message::Plugin(super::PluginMsg::Incoming(incoming));
                if output.send(message).await.is_err() {
                    break;
                }
            }
        })
    })
}

pub fn is_running(id: &str) -> bool {
    processes().lock().is_ok_and(|p| p.contains_key(id))
}

/// Starts a plugin and says hello to it. Messages sent to it right after
/// are queued behind `initialize` in its input, so they need not wait.
pub fn start(plugin: &Installed, init: InitializeParams) -> Result<(), String> {
    let id = plugin.id().to_string();
    if is_running(&id) {
        return Ok(());
    }
    let (program, args) = plugin.program().ok_or_else(|| {
        let p = plugin.manifest.command[0].as_str();
        t!("plugins.err_program_missing", program = p).to_string()
    })?;
    // What it prints for itself goes to its log, new at each start.
    let log = std::fs::File::create(init.data_dir.join("plugin.log"))
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null());
    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(&plugin.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log)
        .env("ARDOISE_PLUGIN_DATA", &init.data_dir);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: only `prctl`, which is async-signal-safe, runs between the
        // fork and the exec. The plugin dies with the app, even killed.
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                Ok(())
            });
        }
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;

    static RUNS: AtomicU64 = AtomicU64::new(0);
    let process = Arc::new(Process {
        stdin: Mutex::new(Some(stdin)),
        pending: Mutex::default(),
        child: Mutex::new(child),
        next_id: AtomicI64::new(1),
        stopping: AtomicBool::new(false),
        run: RUNS.fetch_add(1, Ordering::Relaxed),
    });
    if let Ok(mut all) = processes().lock() {
        all.insert(id.clone(), process.clone());
    }
    let reader = process.clone();
    std::thread::spawn(move || read_loop(id, reader, stdout));
    let request = Rpc::request(
        process.next_id.fetch_add(1, Ordering::Relaxed),
        methods::INITIALIZE,
        init,
    );
    process.send(&request);
    Ok(())
}

fn read_loop(id: String, process: Arc<Process>, stdout: std::process::ChildStdout) {
    let mut input = BufReader::new(stdout);
    let mut reason = None;
    loop {
        match protocol::read_message(&mut input) {
            // A refusal of `initialize` (the first request) is the plugin
            // failing to start.
            Ok(Some(Rpc::Response {
                id: 1,
                result: Err(error),
            })) => {
                reason = Some(error.message);
                break;
            }
            Ok(Some(Rpc::Response { id, result })) => {
                let waiter = process.pending.lock().ok().and_then(|mut p| p.remove(&id));
                if let Some(waiter) = waiter {
                    let _ = waiter.send(result);
                }
            }
            Ok(Some(message)) => emit(Incoming::Rpc(id.clone(), message)),
            Ok(None) => break,
            Err(e) => {
                reason = Some(e.to_string());
                break;
            }
        }
    }
    // The end of its output: the plugin is done, one way or another.
    // Whoever waits for an answer hears that none will come.
    if let Ok(mut pending) = process.pending.lock() {
        pending.clear();
    }
    let status = process.child.lock().ok().and_then(|mut c| {
        let _ = c.kill();
        c.wait().ok()
    });
    let ours = processes()
        .lock()
        .ok()
        .and_then(|mut all| {
            let same = all.get(&id).is_some_and(|p| p.run == process.run);
            same.then(|| all.remove(&id))
        })
        .is_some();
    if ours && !process.stopping.load(Ordering::SeqCst) {
        let why = reason
            .or_else(|| status.map(|s| s.to_string()))
            .unwrap_or_default();
        emit(Incoming::Crashed(id, why));
    }
}

/// Sends a notification to a running plugin; `false` if it is not running.
pub fn notify(id: &str, method: &str, params: impl serde::Serialize) -> bool {
    let process = processes().lock().ok().and_then(|p| p.get(id).cloned());
    process.is_some_and(|p| p.send(&Rpc::notification(method, params)))
}

/// Tells every running plugin something.
pub fn notify_all(method: &str) {
    let all: Vec<_> = processes()
        .lock()
        .map(|p| p.values().cloned().collect())
        .unwrap_or_default();
    for process in all {
        process.send(&Rpc::notification(method, serde_json::Value::Null));
    }
}

/// Asks a running plugin and waits for its answer (at most `wait`).
pub async fn request(
    id: &str,
    method: &str,
    params: impl serde::Serialize,
    wait: Duration,
) -> Result<Value, String> {
    let process = processes()
        .lock()
        .ok()
        .and_then(|p| p.get(id).cloned())
        .ok_or_else(|| t!("plugins.not_running", id = id).to_string())?;
    let request = process.next_id.fetch_add(1, Ordering::Relaxed);
    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Ok(mut pending) = process.pending.lock() {
        pending.insert(request, tx);
    }
    if !process.send(&Rpc::request(request, method, params)) {
        return Err(t!("plugins.not_running", id = id).to_string());
    }
    let answer = tokio::time::timeout(wait, rx).await;
    if let Ok(mut pending) = process.pending.lock() {
        pending.remove(&request);
    }
    match answer {
        Ok(Ok(result)) => result.map_err(|e| e.message),
        // The plugin stopped before it answered.
        Ok(Err(_)) => Err(t!("plugins.not_running", id = id).to_string()),
        Err(_) => Err(t!("plugins.no_answer", id = id).to_string()),
    }
}

/// Answers a request of a plugin.
pub fn respond(id: &str, request: i64, result: Result<Value, RpcError>) {
    let process = processes().lock().ok().and_then(|p| p.get(id).cloned());
    if let Some(process) = process {
        process.send(&Rpc::Response {
            id: request,
            result,
        });
    }
}

/// Asks a plugin to stop, and stops it if it has not within 2 s.
pub fn stop(id: &str) {
    let process = processes().lock().ok().and_then(|mut p| p.remove(id));
    let Some(process) = process else {
        return;
    };
    process.stopping.store(true, Ordering::SeqCst);
    let request = Rpc::request(
        process.next_id.fetch_add(1, Ordering::Relaxed),
        methods::SHUTDOWN,
        Value::Null,
    );
    process.send(&request);
    std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while std::time::Instant::now() < deadline {
            let exited = process
                .child
                .lock()
                .ok()
                .and_then(|mut c| c.try_wait().ok().flatten())
                .is_some();
            if exited {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // Closing its input is the last word; then it is killed.
        if let Ok(mut stdin) = process.stdin.lock() {
            stdin.take();
        }
        if let Ok(mut child) = process.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    });
}

/// Stops every plugin and waits (a little) for them: the app is closing.
pub fn stop_all() {
    let ids: Vec<String> = processes()
        .lock()
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default();
    for id in &ids {
        stop(id);
    }
    if !ids.is_empty() {
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// What the plugins say, read by a test instead of the app's subscription.
#[cfg(test)]
pub fn take_events() -> Option<tokio::sync::mpsc::UnboundedReceiver<Incoming>> {
    channel().1.lock().ok()?.take()
}
