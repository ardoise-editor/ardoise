//! The plugins in the app: starting them, what they ask and say, their
//! panels, commands and items of the status bar, the files and settings
//! they follow, and the installs of the Plugins page.

use std::sync::Arc;

use super::*;
use crate::plugins::{
    host, remote, store, Incoming, Installed, PanelKey, PluginMsg, Registry, Review,
};
use ardoise_plugin::manifest::{Event, Location};
use ardoise_plugin::protocol::{
    self as rpc, codes, methods, ActiveFile, ActiveFileParams, AnnotateParams, CommandParams,
    ConfirmParams, Context, CopyParams, DecorateParams, DiffParams, DiffTabParams, FileParams,
    InitializeParams, Level, MergeParams, Message as Rpc, NotifyParams, OpenFileParams,
    OpenTextParams, PanelParams, PickFileParams, PublishParams, RenameParams, RenderParams,
    ReplaceTextParams, RpcError, SaveFileParams, SearchItemsParams, SetBaseParams, Settings,
    StatusParams, TabDirtyParams, TabParams, UiEvent,
};
use serde_json::Value;

/// A question of a plugin, waiting for the user.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginQuestion {
    pub plugin: String,
    pub request: i64,
    pub message: String,
    pub confirm: Option<String>,
}

async fn install_from(
    repo: String,
    official: bool,
    expect: Option<String>,
) -> Result<Installed, String> {
    let release = remote::latest_release(repo.clone()).await?;
    let bytes = remote::download(release, official).await?;
    let origin = store::Origin {
        repository: Some(repo),
        official,
        linked: None,
    };
    tokio::task::spawn_blocking(move || store::install_archive(&bytes, origin, expect.as_deref()))
        .await
        .map_err(|e| e.to_string())?
}

async fn fetch_review(repo: String) -> Result<Review, String> {
    let release = remote::latest_release(repo.clone()).await?;
    let bytes = remote::download(release, false).await?;
    let manifest = store::manifest_in_archive(&bytes)?;
    if manifest.id.starts_with(store::OFFICIAL_PREFIX) {
        return Err(t!("plugins.err_official_id").to_string());
    }
    Ok(Review {
        manifest,
        repository: repo,
        bytes: Arc::new(bytes),
    })
}

async fn newest_versions(list: Vec<(String, String)>) -> Vec<(String, Result<String, String>)> {
    let mut out = Vec::new();
    for (id, repo) in list {
        out.push((id, remote::latest_release(repo).await.map(|r| r.version)));
    }
    out
}

fn open_url(url: &str) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(program)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn plugin_msg(msg: PluginMsg) -> Message {
    Message::Plugin(msg)
}

impl ArdoiseApp {
    fn plugin_settings(&self) -> Settings {
        Settings {
            locale: rust_i18n::locale().to_string(),
            dark: self.config.dark_mode,
        }
    }

    fn plugin_context(&self) -> Context {
        Context {
            file: self
                .current_file
                .clone()
                .filter(|p| !is_untitled(p) && !is_virtual(p)),
            workspace: self.projects.roots.clone(),
        }
    }

    /// Starts the plugin if it is not running; `false` if it can't be.
    pub(super) fn plugin_ensure(&mut self, id: &str) -> bool {
        if host::is_running(id) {
            return true;
        }
        let Some(plugin) = self.plugins.get(id).cloned() else {
            return false;
        };
        let Some(data_dir) = store::data_dir_of(id) else {
            return false;
        };
        let init = InitializeParams {
            api: rpc::API_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            settings: self.plugin_settings(),
            workspace: self.projects.roots.clone(),
            plugin_dir: plugin.dir.clone(),
            data_dir,
        };
        match host::start(&plugin, init) {
            Ok(()) => {
                self.plugins.crashed.remove(id);
                // The files open now, for the plugins that follow them.
                if plugin.manifest.events.contains(&Event::FileOpened) {
                    for path in self.plugins.told_open.iter() {
                        host::notify(id, methods::FILE_OPENED, FileParams::new(path.clone()));
                    }
                }
                true
            }
            Err(e) => {
                let name = plugin.name();
                self.plugins.crashed.insert(id.to_string(), e.clone());
                let (n, e) = (name.as_str(), e.as_str());
                self.set_status(t!("plugins.err_start", name = n, error = e), true);
                false
            }
        }
    }

    /// At the start of the app: the plugins that follow events start.
    pub(super) fn plugins_start(&mut self) {
        let ids: Vec<String> = self
            .plugins
            .installed
            .iter()
            .filter(|p| p.manifest.starts_with_editor())
            .map(|p| p.id().to_string())
            .collect();
        for id in ids {
            self.plugin_ensure(&id);
        }
        self.plugins.sent_settings = Some(self.plugin_settings());
    }

    /// The language or the theme changed: the running plugins hear of it.
    pub(super) fn plugins_follow_settings(&mut self) {
        let now = self.plugin_settings();
        if self.plugins.sent_settings.as_ref() == Some(&now) {
            return;
        }
        for plugin in &self.plugins.installed {
            host::notify(plugin.id(), methods::SETTINGS_CHANGED, &now);
        }
        self.plugins.sent_settings = Some(now);
    }

    /// A tab of a plugin in front that has nothing to show yet (it came back
    /// with the session): its plugin is asked for it.
    pub(super) fn plugins_follow_tab(&mut self) {
        let Some(path) = self.current_file.clone() else {
            return;
        };
        let Some(tab) = self.plugins.tabs.get(&path).filter(|t| !t.asked).cloned() else {
            return;
        };
        if let Some(t) = self.plugins.tabs.get_mut(&path) {
            t.asked = true;
        }
        if self.plugin_ensure(&tab.plugin) {
            host::notify(
                &tab.plugin,
                methods::PANEL_OPENED,
                PanelParams { panel: tab.tab },
            );
        }
    }

    /// What a tab of a plugin shows.
    pub(super) fn plugin_tab_view<'a>(
        &'a self,
        tab: &crate::plugins::PluginTab,
        dark: bool,
    ) -> Element<'a, Message> {
        let key: PanelKey = (tab.plugin.clone(), tab.tab.clone());
        container(self.plugin_panel_body(&key, dark))
            .padding([10, 14])
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Opens (or brings in front) a dialog of a plugin over the editor.
    fn plugin_open_dialog(&mut self, plugin: &str, tab: String, title: String) -> Task<Message> {
        let key: PanelKey = (plugin.to_string(), tab);
        match self.plugins.dialogs.iter().position(|(k, _)| *k == key) {
            Some(at) => {
                let (k, _) = self.plugins.dialogs.remove(at);
                self.plugins.dialogs.push((k, title));
            }
            None => {
                self.plugins.dialogs.push((key.clone(), title));
                if self.plugin_ensure(plugin) {
                    host::notify(plugin, methods::PANEL_OPENED, PanelParams { panel: key.1 });
                }
            }
        }
        Task::none()
    }

    /// Closes a dialog of a plugin (the plugin hears `tab/closed`, as for a
    /// tab). Whether there was one.
    pub(super) fn plugin_close_dialog(&mut self, key: &PanelKey) -> bool {
        let Some(at) = self.plugins.dialogs.iter().position(|(k, _)| k == key) else {
            return false;
        };
        self.plugins.dialogs.remove(at);
        self.plugins.views.remove(key);
        self.plugins.revealed.retain(|(k, _)| k != key);
        host::notify(
            &key.0,
            methods::TAB_CLOSED,
            PanelParams {
                panel: key.1.clone(),
            },
        );
        true
    }

    /// `base` with the dialogs of the plugins over it, each in the frame of a
    /// popin: the title and a cross, the tabs, the body, the footer.
    pub(super) fn plugin_dialogs_view<'a>(
        &'a self,
        base: Element<'a, Message>,
        dark: bool,
    ) -> Element<'a, Message> {
        let mut shown = base;
        for (key, title) in &self.plugins.dialogs {
            let close = || Message::Plugin(PluginMsg::CloseDialog(key.0.clone(), key.1.clone()));
            let parts = match self.plugins.views.get(key) {
                Some(node) => crate::plugins::render::dialog_parts(&self.plugins, key, node, dark),
                None => crate::plugins::render::DialogParts {
                    tabs: None,
                    body: self.plugin_panel_body(key, dark),
                    footer: None,
                    form: false,
                },
            };
            let mut frame = crate::popin::Frame::new(if parts.form {
                crate::popin::WIDTH_FORM
            } else {
                crate::popin::WIDTH_CONFIRM
            });
            frame.title = Some(title.clone());
            frame.on_close = Some(close());
            frame.tabs = parts.tabs;
            frame.footer = parts.footer;
            let body = container(
                iced::widget::scrollable(parts.body)
                    .width(Length::Fill)
                    .style(se_theme::Scrollbar { dark }.style()),
            )
            .max_height(crate::popin::MAX_BODY);
            let card = crate::popin::frame(dark, frame, body.into());
            shown = crate::floating::modal(shown, card, (!parts.form).then(close), dark, false);
        }
        shown
    }

    /// The dialog of a plugin in front, if any.
    pub(super) fn plugin_front_dialog(&self) -> Option<&PanelKey> {
        self.plugins.dialogs.last().map(|(k, _)| k)
    }

    /// Opens (or brings in front) a tab of a plugin.
    fn plugin_open_tab(&mut self, plugin: &str, p: TabParams) -> Result<Task<Message>, String> {
        let good = !p.tab.is_empty()
            && p.tab.len() <= 64
            && p.tab
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
        if !good || p.title.trim().is_empty() {
            return Err(t!("plugins.err_tab").to_string());
        }
        if p.dialog {
            return Ok(self.plugin_open_dialog(plugin, p.tab, p.title.trim().to_string()));
        }
        let path = crate::plugins::tab_path(plugin, &p.tab, p.title.trim());
        let old = self
            .plugins
            .tabs
            .iter()
            .find(|(_, t)| t.plugin == plugin && t.tab == p.tab)
            .map(|(path, _)| path.clone());
        match old {
            // A new title: the tab is renamed where it is.
            Some(old) if old != path => {
                if let Some(tab) = self.plugins.tabs.remove(&old) {
                    self.plugins.tabs.insert(path.clone(), tab);
                }
                self.sidebar.rename_file(&old, path.clone());
                if self.current_file.as_ref() == Some(&old) {
                    self.current_file = Some(path.clone());
                }
            }
            Some(_) => {}
            None => {
                self.plugins.tabs.insert(
                    path.clone(),
                    crate::plugins::PluginTab {
                        plugin: plugin.to_string(),
                        tab: p.tab,
                        dirty: false,
                        asked: true,
                    },
                );
                self.sidebar.add_file(path.clone());
            }
        }
        Ok(self.activate(path))
    }

    /// The file in front changed: the plugins that follow it hear of it.
    pub(super) fn plugins_follow_active(&mut self) {
        let now = self.plugin_context().file;
        if self.plugins.sent_active.as_ref() == Some(&now) {
            return;
        }
        for plugin in &self.plugins.installed {
            if plugin.manifest.events.contains(&Event::ActiveFile) {
                host::notify(
                    plugin.id(),
                    methods::ACTIVE_FILE_CHANGED,
                    ActiveFileParams { path: now.clone() },
                );
            }
        }
        self.plugins.sent_active = Some(now);
    }

    /// A file was opened, saved or closed: the plugins that asked hear of it.
    pub(super) fn plugins_file_event(&mut self, event: Event, path: &Path) {
        self.plugins_file_event_with(event, path, None);
    }

    /// The same, with what the file held before (a file rewritten by the
    /// editor, one about to be deleted).
    pub(super) fn plugins_file_event_with(
        &mut self,
        event: Event,
        path: &Path,
        previous: Option<String>,
    ) {
        if is_untitled(path) || is_virtual(path) {
            return;
        }
        let method = match event {
            Event::FileOpened => {
                if !self.plugins.told_open.insert(path.to_path_buf()) {
                    return;
                }
                methods::FILE_OPENED
            }
            Event::FileClosed => {
                if !self.plugins.told_open.remove(path) {
                    return;
                }
                methods::FILE_CLOSED
            }
            Event::FileSaved => methods::FILE_SAVED,
            Event::FileDeleted => methods::FILE_DELETED,
            Event::Startup | Event::ActiveFile | Event::FileRenamed => return,
        };
        let params = FileParams {
            path: path.to_path_buf(),
            previous,
            options: Vec::new(),
        };
        for plugin in &self.plugins.installed {
            if plugin.manifest.events.contains(&event) {
                host::notify(plugin.id(), method, &params);
            }
        }
    }

    /// Whether a plugin follows the files deleted (it is told what they held
    /// before they go: reading them costs only then).
    pub(super) fn plugins_follow(&self, event: Event) -> bool {
        self.plugins
            .installed
            .iter()
            .any(|p| p.manifest.events.contains(&event))
    }

    /// A file or folder about to be deleted: the plugins that follow it are
    /// told what each file in it holds (for a folder, its first files), and
    /// which of their boxes were left ticked (by plugin).
    pub(super) fn plugins_before_delete(
        &mut self,
        path: &Path,
        chosen: &std::collections::BTreeMap<String, Vec<String>>,
    ) {
        if !self.plugins_follow(Event::FileDeleted) {
            return;
        }
        let files: Vec<PathBuf> = if path.is_dir() {
            walk_files(path, 2000)
        } else {
            vec![path.to_path_buf()]
        };
        for file in files {
            if is_untitled(&file) || is_virtual(&file) {
                continue;
            }
            let text = std::fs::read(&file)
                .ok()
                .filter(|b| b.len() <= 5 * 1024 * 1024)
                .and_then(|b| String::from_utf8(b).ok());
            for plugin in &self.plugins.installed {
                if !plugin.manifest.events.contains(&Event::FileDeleted) {
                    continue;
                }
                let params = FileParams {
                    path: file.clone(),
                    previous: text.clone(),
                    options: chosen.get(plugin.id()).cloned().unwrap_or_default(),
                };
                host::notify(plugin.id(), methods::FILE_DELETED, &params);
            }
        }
    }

    /// Asks before a file is deleted, with the boxes the plugins that follow
    /// deletions add (each has 2 seconds to answer; one that can't adds
    /// none).
    pub(super) fn plugins_ask_delete(&mut self, path: PathBuf) -> Task<Message> {
        let asked: Vec<String> = self
            .plugins
            .installed
            .iter()
            .filter(|p| p.manifest.events.contains(&Event::FileDeleted))
            .map(|p| p.id().to_string())
            .collect();
        if asked.is_empty() || path.is_dir() {
            self.overlay = ActiveOverlay::ConfirmFileDelete(path, Vec::new());
            return Task::none();
        }
        Task::perform(
            async move {
                let mut options = Vec::new();
                for id in asked {
                    let answer = host::request(
                        &id,
                        methods::DELETE_OPTIONS,
                        FileParams::new(path.clone()),
                        std::time::Duration::from_secs(2),
                    )
                    .await;
                    let given = answer
                        .ok()
                        .and_then(|v| {
                            serde_json::from_value::<ardoise_plugin::protocol::DeleteOptions>(v)
                                .ok()
                        })
                        .unwrap_or_default();
                    options.extend(given.options.into_iter().map(|o| (id.clone(), o)));
                }
                (path, options)
            },
            |(path, options)| Message::Plugin(PluginMsg::DeleteOptions(path, options)),
        )
    }

    /// A file or folder was renamed or moved.
    pub(super) fn plugins_renamed(&self, from: &Path, to: &Path) {
        let params = RenameParams {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
        };
        for plugin in &self.plugins.installed {
            if plugin.manifest.events.contains(&Event::FileRenamed) {
                host::notify(plugin.id(), methods::FILE_RENAMED, &params);
            }
        }
    }

    /// The problems a plugin found in a file: shown with the others, and the
    /// file marked in the tree and the tabs.
    fn plugin_published(&mut self, id: String, p: PublishParams) {
        let list: Vec<crate::lint::Diagnostic> = p
            .diagnostics
            .into_iter()
            .map(crate::lint::Diagnostic::from)
            .collect();
        let by_plugin = self.plugins.published.entry(p.path.clone()).or_default();
        if list.is_empty() {
            by_plugin.remove(&id);
        } else {
            by_plugin.insert(id, list);
        }
        if by_plugin.is_empty() {
            self.plugins.published.remove(&p.path);
        }
        let errors = self
            .plugins
            .published_for(&p.path)
            .iter()
            .filter(|d| d.severity == crate::lint::Severity::Error)
            .count();
        if self.lint.file.as_ref() == Some(&p.path) {
            self.lint.force = true;
        } else {
            self.file_marks.set_errors(&p.path, errors);
        }
    }

    /// Runs a command of an official plugin, or says it is not installed.
    pub(super) fn run_official_command(
        &mut self,
        id: &str,
        command: &str,
        missing: String,
    ) -> Task<Message> {
        if self.plugins.get(id).is_none() {
            self.set_status(missing, true);
            return Task::none();
        }
        self.plugin_update(PluginMsg::Run(id.to_string(), command.to_string()))
    }

    /// The tools a user had before the plugins: worked out once, at the
    /// first start of a version with plugins (a new install has none).
    pub(super) fn plugins_carry(&mut self) -> Task<Message> {
        if !self.config.plugins_carried {
            self.config.plugins_carried = true;
            self.config.plugins_to_carry = crate::plugins::migrate::relied_on(&self.config)
                .into_iter()
                .map(str::to_string)
                .collect();
            self.config_dirty = true;
        }
        let installed = |id: &String| self.plugins.get(id).is_some();
        let left: Vec<String> = self
            .config
            .plugins_to_carry
            .iter()
            .filter(|id| !installed(id))
            .cloned()
            .collect();
        if left.len() != self.config.plugins_to_carry.len() {
            self.config.plugins_to_carry = left.clone();
            self.config_dirty = true;
        }
        if left.is_empty() {
            return Task::none();
        }
        self.set_status(t!("plugins.carrying", n = left.len()), false);
        self.plugin_update(PluginMsg::LoadRegistry)
    }

    /// The registry is there: the tools still to carry over install.
    fn plugins_carry_on(&mut self) -> Task<Message> {
        if !matches!(self.plugins.registry, Registry::Loaded(_)) {
            return Task::none();
        }
        let ids: Vec<String> = self
            .config
            .plugins_to_carry
            .iter()
            .filter(|id| self.plugins.get(id).is_none())
            .cloned()
            .collect();
        Task::batch(
            ids.into_iter()
                .map(|id| self.plugin_update(PluginMsg::InstallOfficial(id))),
        )
    }

    /// A command of the Git plugin (it is said when the plugin is missing).
    pub(super) fn git_command(&mut self, command: &str) -> Task<Message> {
        let missing = t!("git.plugin_missing").to_string();
        self.run_official_command(crate::plugins::migrate::GIT, command, missing)
    }

    /// A text a plugin shows in a tab of its own, to read only (`rev://`);
    /// the path of that tab.
    fn plugin_open_text(&mut self, p: OpenTextParams) -> PathBuf {
        self.untitled_counter += 1;
        let path = PathBuf::from(format!("rev://{}/{}", self.untitled_counter, p.title));
        let language =
            crate::editor::extension_to_token(&p.extension.to_lowercase()).map(str::to_string);
        self.stash_current();
        self.sidebar.add_file(path.clone());
        let loaded = Loaded {
            path: path.clone(),
            text: p.text,
            format: TextFormat::default(),
            stamp: None,
        };
        self.load_buffer(path.clone(), CachedBuffer::from_loaded(loaded, language));
        path
    }

    /// A file in conflict a plugin gives, to resolve in a merge tab (none:
    /// there is no conflict in the text).
    fn plugin_merge(&mut self, plugin: String, p: MergeParams) -> Option<Task<Message>> {
        let merge = crate::diff::conflicts::parse(&p.text, p.swap)?;
        let dir = p.path.parent().map(Path::to_path_buf).unwrap_or_default();
        let name = p.path.file_name()?.to_string_lossy().to_string();
        let existing = self
            .merge_tabs
            .iter()
            .find(|(_, t)| {
                t.owner
                    .as_ref()
                    .is_some_and(|(o, tab)| *o == plugin && *tab == p.tab)
            })
            .map(|(path, _)| path.clone());
        let path = existing.unwrap_or_else(|| {
            self.merge_counter += 1;
            PathBuf::from(format!("merge://{}/{}", self.merge_counter, name))
        });
        let mut tab = crate::diff_view::merge::MergeTab::new(dir, name, merge);
        tab.owner = Some((plugin, p.tab));
        self.merge_tabs.insert(path.clone(), tab);
        self.sidebar.add_file(path.clone());
        Some(self.activate(path))
    }

    /// Diffs a plugin gives, in a diff tab of its own (the one it had, if
    /// the id is the same: what it shows is replaced, where it is).
    fn plugin_diff_tab(&mut self, plugin: String, p: DiffTabParams) -> Task<Message> {
        use ardoise_plugin::protocol::DiffMode;
        let mut entries = Vec::new();
        let mut diffs = Vec::new();
        let mut files = Vec::new();
        for f in p.files {
            let parsed = crate::diff::parse::parse_diff(&f.patch)
                .into_iter()
                .next()
                .unwrap_or_else(|| crate::diff::FileDiff {
                    path: f.path.clone(),
                    ..crate::diff::FileDiff::default()
                });
            let source = match f.mode {
                DiffMode::Plain => crate::diff::Source::Plain,
                DiffMode::Unstaged => crate::diff::Source::Unstaged,
                DiffMode::Staged => crate::diff::Source::Staged,
                DiffMode::Commit => crate::diff::Source::Commit("HEAD".into()),
                DiffMode::Against => crate::diff::Source::Tree("HEAD".into()),
            };
            entries.push(crate::diff_view::Entry {
                source,
                change: crate::diff::Change {
                    path: f.path,
                    from: f.from,
                },
                staged: f.mode == DiffMode::Staged,
            });
            diffs.push(parsed);
            files.push(f.file);
        }
        let owner = Some((plugin.clone(), p.tab.clone(), files));
        let existing = self
            .diff_tabs
            .iter()
            .find(|(_, t)| {
                t.owner
                    .as_ref()
                    .is_some_and(|(o, tab, _)| *o == plugin && *tab == p.tab)
            })
            .map(|(path, _)| path.clone());
        let start = p.start.min(entries.len().saturating_sub(1));
        if let Some(path) = existing {
            // The same tab: its diffs are replaced, the file in view kept.
            if let Some(tab) = self.diff_tabs.get_mut(&path) {
                let index = tab
                    .viewer
                    .index
                    .unwrap_or(start)
                    .min(entries.len().saturating_sub(1));
                let change = tab.viewer.change;
                tab.viewer.entries = entries;
                tab.viewer.provided = Some(diffs);
                tab.owner = owner;
                let task = tab.viewer.open(index, crate::diff_view::Land::First);
                tab.viewer.change = change.min(tab.viewer.changes().saturating_sub(1));
                return task;
            }
        }
        self.diff_counter += 1;
        let path = PathBuf::from(format!("diff://{}/{}", self.diff_counter, p.title));
        let mut viewer = crate::diff_view::Viewer::default();
        viewer.entries = entries;
        viewer.provided = Some(diffs);
        let load = viewer.open(start, crate::diff_view::Land::First);
        self.diff_tabs
            .insert(path.clone(), super::DiffTabState { viewer, owner });
        self.sidebar.add_file(path.clone());
        let show = self.activate(path);
        Task::batch([show, load])
    }

    /// What the plugins say of how files stand, in the tree and the tabs.
    fn plugins_decorate(&mut self) {
        use ardoise_plugin::protocol::FileState as S;
        let states = self
            .plugins
            .decorations
            .values()
            .flatten()
            .map(|d| {
                let state = match d.state {
                    S::Modified => crate::diff::FileState::Modified,
                    S::Added => crate::diff::FileState::Added,
                    S::Untracked => crate::diff::FileState::Untracked,
                    S::Conflicted => crate::diff::FileState::Conflicted,
                    S::Ignored => crate::diff::FileState::Ignored,
                };
                (d.path.clone(), state, d.folder)
            })
            .collect();
        self.file_marks.set_git(states);
    }

    /// The text the file in front is compared with came from a plugin.
    pub(super) fn plugin_base_loaded(&mut self, path: PathBuf, base: crate::plugins::Base) {
        match base {
            crate::plugins::Base::Text(text) => self.marks_base_loaded(path, Some(text)),
            crate::plugins::Base::None => self.marks_base_loaded(path, None),
        }
    }

    /// A column of annotations of a plugin (a blame), or none.
    fn plugin_annotate(&mut self, plugin: String, p: AnnotateParams) {
        let Some(found) = p.annotations else {
            if self.plugins.annotations_of.as_deref() == Some(plugin.as_str())
                && self.blame.as_ref().is_some_and(|b| b.path == p.path)
            {
                self.blame = None;
                self.plugins.annotations_of = None;
            }
            return;
        };
        let dark = self.config.dark_mode;
        let palette = se_theme::palette(dark);
        let lines = found
            .lines
            .into_iter()
            .map(|l| crate::code_editor::Annotation {
                text: l.text,
                tint: l.tint.map_or(iced::Color::TRANSPARENT, |[r, g, b]| {
                    iced::Color::from_rgba8(r, g, b, 0.16)
                }),
                details: l.detail,
            })
            .collect();
        self.blame = Some(super::BlameState {
            path: p.path,
            annotations: crate::code_editor::Annotations {
                lines: Arc::new(lines),
                text: palette.muted,
                details: Arc::new(found.details),
                link: palette.accent,
                card_text: palette.text,
                card_background: palette.elevated,
                card_border: palette.border_strong,
            },
        });
        self.plugins.annotations_of = Some(plugin);
    }

    /// A click on a line of the annotations of a plugin: it hears of it.
    pub(super) fn plugin_annotation_clicked(&mut self, line: usize) -> bool {
        let (Some(plugin), Some(blame)) =
            (self.plugins.annotations_of.clone(), self.blame.as_ref())
        else {
            return false;
        };
        host::notify(
            &plugin,
            methods::ANNOTATION_CLICKED,
            ardoise_plugin::protocol::LineParams {
                path: blame.path.clone(),
                line,
            },
        );
        true
    }

    /// The settings of a linter changed: the file is looked at again.
    fn plugin_settings_touched(&mut self, plugin: &str, panel: &str) {
        let linter = self
            .plugins
            .get(plugin)
            .is_some_and(|p| p.manifest.contributes.linter.is_some());
        if linter
            && self
                .plugins
                .settings_panel(plugin)
                .is_some_and(|p| p.id == panel)
        {
            self.lint.force = true;
        }
    }

    fn open_plugin_panel(&mut self, key: &PanelKey) {
        if self.plugin_ensure(&key.0) {
            host::notify(
                &key.0,
                methods::PANEL_OPENED,
                PanelParams {
                    panel: key.1.clone(),
                },
            );
        }
    }

    fn close_plugin_panel(&mut self, key: &PanelKey) {
        host::notify(
            &key.0,
            methods::PANEL_CLOSED,
            PanelParams {
                panel: key.1.clone(),
            },
        );
    }

    /// Opens or closes a panel of a plugin.
    fn toggle_plugin_panel(&mut self, key: PanelKey) {
        let location = self
            .plugins
            .get(&key.0)
            .and_then(|p| p.manifest.contributes.panels.iter().find(|x| x.id == key.1))
            .map(|p| p.location);
        match location {
            Some(Location::Sidebar) => {
                if let Some(old) = self.plugins.sidebar.take() {
                    self.close_plugin_panel(&old);
                    if old == key {
                        return;
                    }
                }
                // It takes the place of the panels of the app.
                self.panels = crate::sidebar::Panels {
                    files: false,
                    projects: false,
                };
                self.show_sidebar = true;
                self.open_plugin_panel(&key);
                self.plugins.sidebar = Some(key);
            }
            Some(Location::Bottom) => {
                if let Some(old) = self.plugins.bottom.take() {
                    self.close_plugin_panel(&old);
                    if old == key {
                        return;
                    }
                }
                self.open_plugin_panel(&key);
                // One tool panel at the bottom: those of the app step aside.
                self.scratch.open = false;
                self.bookmark_panel.open = false;
                self.plugins.bottom = Some(key);
            }
            Some(Location::Settings) => {
                let _ = self.plugin_update(PluginMsg::ToggleSettings(key.0));
            }
            None => {}
        }
    }

    /// A tool panel of the app opens under the editor: the plugin's one
    /// steps aside (one tool panel at the bottom), and its screen lets the
    /// keyboard go.
    pub(super) fn plugin_bottom_yield(&mut self) {
        if let Some(old) = self.plugins.bottom.take() {
            self.close_plugin_panel(&old);
        }
        self.plugins.screen_focus = None;
    }

    /// A panel of the app was opened in the sidebar: the plugin's one closes.
    pub(super) fn plugin_sidebar_yield(&mut self) {
        if let Some(old) = self.plugins.sidebar.take() {
            self.close_plugin_panel(&old);
        }
    }

    pub(super) fn plugin_update(&mut self, msg: PluginMsg) -> Task<Message> {
        match msg {
            PluginMsg::Incoming(Incoming::Rpc(id, message)) => self.plugin_incoming(id, message),
            PluginMsg::Incoming(Incoming::Crashed(id, why)) => {
                let name = self
                    .plugins
                    .get(&id)
                    .map(|p| p.name())
                    .unwrap_or(id.clone());
                self.plugins.forget(&id);
                if matches!(&self.overlay, ActiveOverlay::PluginConfirm(q) if q.plugin == id) {
                    self.overlay = ActiveOverlay::None;
                }
                self.plugins.crashed.insert(id, why.clone());
                let (n, e) = (name.as_str(), why.as_str());
                self.set_status(t!("plugins.crashed", name = n, error = e), true);
                Task::none()
            }
            PluginMsg::TogglePanel(plugin, panel) => {
                self.toggle_plugin_panel((plugin, panel));
                Task::none()
            }
            PluginMsg::CloseBottom => {
                self.plugin_bottom_yield();
                Task::none()
            }
            PluginMsg::Ui(plugin, panel, action, value) => {
                self.plugin_settings_touched(&plugin, &panel);
                // The panel clicked in: its keys work now.
                self.plugins.panel_focus = Some((plugin.clone(), panel.clone()));
                host::notify(
                    &plugin,
                    methods::UI_EVENT,
                    UiEvent {
                        panel,
                        action,
                        value,
                    },
                );
                Task::none()
            }
            PluginMsg::EditorAction(plugin, panel, id, action) => {
                let read_only = self
                    .plugins
                    .views
                    .get(&(plugin.clone(), panel.clone()))
                    .is_some_and(|root| editor_read_only(root, &id));
                if read_only && action.is_edit() {
                    return Task::none();
                }
                let slot = ((plugin.clone(), panel.clone()), id.clone());
                let Some((content, _)) = self.plugins.editors.get_mut(&slot) else {
                    return Task::none();
                };
                let edit = action.is_edit();
                let typed = match &action {
                    iced::widget::text_editor::Action::Edit(
                        iced::widget::text_editor::Edit::Insert(c),
                    ) => Some(*c),
                    _ => None,
                };
                content.perform(action);
                let touched = self.plugin_editor_touched(slot.clone(), typed);
                let Some((content, _)) = self.plugins.editors.get_mut(&slot) else {
                    return touched;
                };
                if edit {
                    let text = content.text();
                    host::notify(
                        &plugin,
                        methods::UI_EVENT,
                        UiEvent {
                            panel,
                            action: id,
                            value: Some(Value::String(text)),
                        },
                    );
                }
                touched
            }
            PluginMsg::Reveal(plugin, panel, field) => {
                let slot = ((plugin, panel), field);
                if !self.plugins.revealed.remove(&slot) {
                    self.plugins.revealed.insert(slot);
                }
                Task::none()
            }
            PluginMsg::CloseDialog(plugin, panel) => {
                self.plugin_close_dialog(&(plugin, panel));
                Task::none()
            }
            PluginMsg::Input(plugin, panel, field, value) => {
                self.plugin_settings_touched(&plugin, &panel);
                // A field has the keyboard: its keys are its own.
                self.plugins.panel_focus = None;
                self.plugins.inputs.insert(
                    ((plugin.clone(), panel.clone()), field.clone()),
                    value.clone(),
                );
                host::notify(
                    &plugin,
                    methods::UI_EVENT,
                    UiEvent {
                        panel,
                        action: field,
                        value: Some(Value::String(value)),
                    },
                );
                Task::none()
            }
            PluginMsg::Run(plugin, command) => {
                if self.plugin_ensure(&plugin) {
                    let context = self.plugin_context();
                    host::notify(
                        &plugin,
                        methods::COMMAND,
                        CommandParams { command, context },
                    );
                }
                Task::none()
            }
            PluginMsg::RunAt(plugin, command, path) => {
                if self.plugin_ensure(&plugin) {
                    let context = Context {
                        file: Some(path),
                        workspace: self.projects.roots.clone(),
                    };
                    host::notify(
                        &plugin,
                        methods::COMMAND,
                        CommandParams { command, context },
                    );
                }
                Task::none()
            }
            PluginMsg::Respond(plugin, request, result) => {
                host::respond(&plugin, request, result);
                Task::none()
            }
            PluginMsg::DeleteOptions(path, options) => {
                self.overlay = ActiveOverlay::ConfirmFileDelete(path, options);
                Task::none()
            }
            PluginMsg::DeleteOptionToggled(at, on) => {
                if let ActiveOverlay::ConfirmFileDelete(_, options) = &mut self.overlay {
                    if let Some((_, option)) = options.get_mut(at) {
                        option.checked = on;
                    }
                }
                Task::none()
            }
            PluginMsg::Answer(plugin, request, yes) => {
                self.overlay = ActiveOverlay::None;
                host::respond(&plugin, request, Ok(Value::Bool(yes)));
                Task::none()
            }
            PluginMsg::ScreenFocus(plugin, panel, id, on) => {
                self.plugins.screen_focus = on.then_some(((plugin, panel), id));
                self.plugins.panel_focus = None;
                Task::none()
            }
            PluginMsg::ScreenCopy(text) => iced::clipboard::write(text),
            PluginMsg::ScreenPaste(plugin, panel, id, bracketed) => {
                iced::clipboard::read().map(move |text| {
                    plugin_msg(PluginMsg::ScreenPasted(
                        plugin.clone(),
                        panel.clone(),
                        id.clone(),
                        bracketed,
                        text,
                    ))
                })
            }
            PluginMsg::ScreenPasted(plugin, panel, id, bracketed, text) => {
                // No escape sequence may come from a paste: `ESC[201~` would
                // end the bracketed paste and what follows would run.
                let Some(text) = text.map(|t| paste_text_safe(&t)) else {
                    return Task::none();
                };
                let bytes = if bracketed {
                    format!("\x1b[200~{text}\x1b[201~").into_bytes()
                } else {
                    // A pasted line break is the Enter key.
                    text.replace("\r\n", "\r").replace('\n', "\r").into_bytes()
                };
                let input = ardoise_plugin::ui::ScreenInput::Input { bytes };
                self.plugin_update(PluginMsg::Ui(
                    plugin,
                    panel,
                    id,
                    serde_json::to_value(input).ok(),
                ))
            }
            PluginMsg::Grid(plugin, panel, id, m) => self.plugin_grid(plugin, panel, id, m),
            PluginMsg::GridResize(x) => {
                self.plugin_grid_resize(x);
                Task::none()
            }
            PluginMsg::GridResizeEnd => {
                self.plugin_grid_resize_end();
                Task::none()
            }
            PluginMsg::GridPasted(text) => {
                self.plugin_grid_pasted(text);
                Task::none()
            }
            PluginMsg::Completed(slot, at, result) => {
                self.plugin_completed(slot, at, result);
                Task::none()
            }
            PluginMsg::CompletionPick(index) => self.plugin_completion_accept(index),
            PluginMsg::ScreenOpen(word, dir) => {
                let dir = if dir.as_os_str().is_empty() {
                    self.projects.roots.first().cloned().unwrap_or_default()
                } else {
                    dir
                };
                match crate::places::location_with(&word, &dir, &mut |p| p.is_file()) {
                    Some(at) => self.open_place(at),
                    None => Task::none(),
                }
            }
            PluginMsg::ToggleSettings(id) => {
                let Some(panel) = self.plugins.settings_panel(&id).map(|p| p.id.clone()) else {
                    return Task::none();
                };
                let key = (id, panel);
                if let Some(old) = self.plugins.settings_open.take() {
                    self.close_plugin_panel(&old);
                    if old == key {
                        return Task::none();
                    }
                }
                self.open_plugin_panel(&key);
                self.plugins.settings_open = Some(key);
                Task::none()
            }
            PluginMsg::Restart(id) => {
                if self.plugin_ensure(&id) {
                    let open: Vec<PanelKey> = [
                        self.plugins.sidebar.clone(),
                        self.plugins.bottom.clone(),
                        self.plugins.settings_open.clone(),
                    ]
                    .into_iter()
                    .flatten()
                    .filter(|k| k.0 == id)
                    .collect();
                    for key in open {
                        host::notify(&key.0, methods::PANEL_OPENED, PanelParams { panel: key.1 });
                    }
                }
                Task::none()
            }
            PluginMsg::LoadRegistry => {
                if matches!(self.plugins.registry, Registry::Loading) {
                    return Task::none();
                }
                self.plugins.registry = Registry::Loading;
                Task::perform(remote::fetch_registry(), |r| {
                    plugin_msg(PluginMsg::RegistryLoaded(r))
                })
            }
            PluginMsg::RegistryLoaded(result) => {
                self.plugins.registry = match result {
                    Ok(list) => Registry::Loaded(list),
                    Err(e) => Registry::Failed(e),
                };
                self.plugins_carry_on()
            }
            PluginMsg::InstallOfficial(id) => {
                let repo = match &self.plugins.registry {
                    Registry::Loaded(list) => list
                        .iter()
                        .find(|l| l.id == id)
                        .map(|l| l.repository.clone()),
                    _ => None,
                }
                .or_else(|| {
                    self.plugins
                        .get(&id)
                        .filter(|p| p.origin.official)
                        .and_then(|p| p.origin.repository.clone())
                });
                let Some(repo) = repo else {
                    return Task::none();
                };
                if !self.plugins.busy.insert(id.clone()) {
                    return Task::none();
                }
                self.plugins.errors.remove(&id);
                host::stop(&id);
                let key = id.clone();
                Task::perform(install_from(repo, true, Some(id)), move |r| {
                    plugin_msg(PluginMsg::Installed(key.clone(), Box::new(r)))
                })
            }
            PluginMsg::Update(id) => {
                let Some(plugin) = self.plugins.get(&id) else {
                    return Task::none();
                };
                if plugin.origin.official {
                    return self.plugin_update(PluginMsg::InstallOfficial(id));
                }
                let Some(repo) = plugin.origin.repository.clone() else {
                    return Task::none();
                };
                if !self.plugins.busy.insert(id.clone()) {
                    return Task::none();
                }
                self.plugins.errors.remove(&id);
                host::stop(&id);
                let key = id.clone();
                Task::perform(install_from(repo, false, Some(id)), move |r| {
                    plugin_msg(PluginMsg::Installed(key.clone(), Box::new(r)))
                })
            }
            PluginMsg::Installed(key, result) => {
                let result = *result;
                self.plugins.busy.remove(&key);
                match result {
                    Ok(installed) => {
                        let id = installed.id().to_string();
                        // One of the tools the user had before the plugins.
                        if self.config.plugins_to_carry.contains(&id) {
                            self.config.plugins_to_carry.retain(|p| *p != id);
                            self.config_dirty = true;
                        }
                        self.plugins.errors.remove(&key);
                        self.plugins.updates.remove(&id);
                        self.plugins.forget(&id);
                        self.plugins.reload();
                        let name = installed.name();
                        let (n, v) = (name.as_str(), installed.manifest.version.as_str());
                        self.set_status(
                            t!("plugins.installed_status", name = n, version = v),
                            false,
                        );
                        // Panels of a plugin that is no longer there close.
                        let has = |k: &PanelKey| {
                            installed
                                .manifest
                                .contributes
                                .panels
                                .iter()
                                .any(|p| p.id == k.1)
                        };
                        for slot in [
                            &mut self.plugins.sidebar,
                            &mut self.plugins.bottom,
                            &mut self.plugins.settings_open,
                        ] {
                            if slot.as_ref().is_some_and(|k| k.0 == id && !has(k)) {
                                *slot = None;
                            }
                        }
                        if installed.manifest.starts_with_editor()
                            || self.plugins.has_open_panel(&id)
                        {
                            return self.plugin_update(PluginMsg::Restart(id));
                        }
                    }
                    Err(e) => {
                        self.plugins.errors.insert(key, e);
                    }
                }
                Task::none()
            }
            PluginMsg::AskUninstall(id) => {
                self.overlay = ActiveOverlay::ConfirmPluginUninstall(id, false);
                Task::none()
            }
            PluginMsg::UninstallDataToggled(on) => {
                if let ActiveOverlay::ConfirmPluginUninstall(_, with_data) = &mut self.overlay {
                    *with_data = on;
                }
                Task::none()
            }
            PluginMsg::UninstallWithData(id) => {
                self.overlay = ActiveOverlay::None;
                if !self.plugins.busy.insert(id.clone()) {
                    return Task::none();
                }
                // Started if it is not: only it knows what it keeps outside
                // of its folder.
                if !self.plugin_ensure(&id) {
                    return self.plugin_update(PluginMsg::DataForgotten(id, Ok(())));
                }
                let asked = id.clone();
                Task::perform(
                    async move {
                        host::request(
                            &asked,
                            methods::FORGET_DATA,
                            serde_json::Value::Null,
                            std::time::Duration::from_secs(30),
                        )
                        .await
                        .map(|_| ())
                    },
                    move |r| plugin_msg(PluginMsg::DataForgotten(id.clone(), r)),
                )
            }
            PluginMsg::DataForgotten(id, result) => {
                self.plugins.busy.remove(&id);
                let task = self.plugin_update(PluginMsg::Uninstall(id.clone()));
                if self.plugins.get(&id).is_some() {
                    // The program could not be removed: its data stays too.
                    return task;
                }
                match (store::remove_data(&id), result) {
                    (Err(e), _) => {
                        self.set_status(format!("{}: {e}", t!("status.error")), true);
                    }
                    // A plugin that does not know the request keeps nothing
                    // outside of its folder.
                    (Ok(()), Err(e)) if !e.contains("-32601") && !e.contains("not found") => {
                        self.set_status(t!("plugins.data_forget_failed", error = e.as_str()), true);
                    }
                    (Ok(()), _) => self.set_status(t!("plugins.data_removed"), false),
                }
                task
            }
            PluginMsg::Uninstall(id) => {
                self.overlay = ActiveOverlay::None;
                host::stop(&id);
                match store::uninstall(&id) {
                    Ok(()) => {
                        self.plugins.forget(&id);
                        self.plugins.crashed.remove(&id);
                        self.plugins.updates.remove(&id);
                        self.plugins.errors.remove(&id);
                        for slot in [
                            &mut self.plugins.sidebar,
                            &mut self.plugins.bottom,
                            &mut self.plugins.settings_open,
                        ] {
                            if slot.as_ref().is_some_and(|k| k.0 == id) {
                                *slot = None;
                            }
                        }
                        self.plugins.reload();
                    }
                    Err(e) => {
                        self.plugins.errors.insert(id, e);
                    }
                }
                Task::none()
            }
            PluginMsg::CheckUpdates => {
                let list: Vec<(String, String)> = self
                    .plugins
                    .installed
                    .iter()
                    .filter(|p| p.origin.linked.is_none())
                    .filter_map(|p| Some((p.id().to_string(), p.origin.repository.clone()?)))
                    .collect();
                if list.is_empty() {
                    self.plugins.update_all = false;
                    return Task::none();
                }
                if self.plugins.checking_updates {
                    return Task::none();
                }
                self.plugins.checking_updates = true;
                Task::perform(newest_versions(list), |r| {
                    plugin_msg(PluginMsg::UpdatesChecked(r))
                })
            }
            PluginMsg::UpdatesChecked(results) => {
                self.plugins.checking_updates = false;
                for (id, result) in results {
                    match result {
                        Ok(version) => {
                            let current = self.plugins.get(&id).map(|p| p.manifest.version.clone());
                            if current.is_some_and(|c| remote::is_newer(&version, &c)) {
                                self.plugins.updates.insert(id, version);
                            } else {
                                self.plugins.updates.remove(&id);
                            }
                        }
                        Err(e) => {
                            self.plugins.errors.insert(id, e);
                        }
                    }
                }
                // Asked with the app's update: the newer ones install now.
                if std::mem::take(&mut self.plugins.update_all) {
                    let newer: Vec<String> = self.plugins.updates.keys().cloned().collect();
                    return Task::batch(
                        newer
                            .into_iter()
                            .map(|id| self.plugin_update(PluginMsg::Update(id))),
                    );
                }
                Task::none()
            }
            PluginMsg::RepoTyped(text) => {
                self.plugins.repo_input = text;
                self.plugins.errors.remove("repo");
                Task::none()
            }
            PluginMsg::FetchRepo => {
                let Some(repo) = remote::parse_repo(&self.plugins.repo_input) else {
                    self.plugins
                        .errors
                        .insert("repo".into(), t!("plugins.err_repo").to_string());
                    return Task::none();
                };
                // An official plugin typed by its address installs as one.
                if let Registry::Loaded(list) = &self.plugins.registry {
                    if let Some(listed) = list
                        .iter()
                        .find(|l| l.repository.eq_ignore_ascii_case(&repo))
                    {
                        let id = listed.id.clone();
                        self.plugins.repo_input.clear();
                        return self.plugin_update(PluginMsg::InstallOfficial(id));
                    }
                }
                if !self.plugins.busy.insert("repo".into()) {
                    return Task::none();
                }
                self.plugins.errors.remove("repo");
                Task::perform(fetch_review(repo), |r| {
                    plugin_msg(PluginMsg::Fetched(Box::new(r)))
                })
            }
            PluginMsg::Fetched(result) => {
                let result = *result;
                self.plugins.busy.remove("repo");
                match result {
                    Ok(review) => self.plugins.review = Some(review),
                    Err(e) => {
                        self.plugins.errors.insert("repo".into(), e);
                    }
                }
                Task::none()
            }
            PluginMsg::AcceptReview => {
                let Some(review) = self.plugins.review.take() else {
                    return Task::none();
                };
                let id = review.manifest.id.clone();
                self.plugins.busy.insert("repo".into());
                host::stop(&id);
                let origin = store::Origin {
                    repository: Some(review.repository.clone()),
                    official: false,
                    linked: None,
                };
                let bytes = review.bytes.clone();
                self.plugins.repo_input.clear();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            store::install_archive(&bytes, origin, None)
                        })
                        .await
                        .map_err(|e| e.to_string())?
                    },
                    |r| plugin_msg(PluginMsg::Installed("repo".into(), Box::new(r))),
                )
            }
            PluginMsg::CancelReview => {
                self.plugins.review = None;
                Task::none()
            }
            PluginMsg::LinkFolder => {
                Task::perform(pick_folder(), |p| plugin_msg(PluginMsg::LinkPicked(p)))
            }
            PluginMsg::LinkPicked(None) => Task::none(),
            PluginMsg::LinkPicked(Some(folder)) => {
                #[cfg(unix)]
                let linked = store::link_folder(&folder);
                #[cfg(not(unix))]
                let linked: Result<Installed, String> = Err(t!("plugins.err_platform").to_string());
                if let Ok(installed) = &linked {
                    host::stop(installed.id());
                }
                self.plugin_update(PluginMsg::Installed("link".into(), Box::new(linked)))
            }
            PluginMsg::OpenGuide => {
                open_url(remote::GUIDE_URL);
                Task::none()
            }
        }
    }

    fn plugin_incoming(&mut self, id: String, message: Rpc) -> Task<Message> {
        match message {
            Rpc::Notification { method, params } => {
                let mut follow = Task::none();
                match method.as_str() {
                    methods::RENDER => {
                        if let Ok(p) = serde_json::from_value::<RenderParams>(params) {
                            // Only the panels it has.
                            let known = self.plugins.get(&id).is_some_and(|x| {
                                x.manifest
                                    .contributes
                                    .panels
                                    .iter()
                                    .any(|q| q.id == p.panel)
                            }) || self
                                .plugins
                                .tabs
                                .values()
                                .any(|t| t.plugin == id && t.tab == p.panel)
                                || self
                                    .plugins
                                    .dialogs
                                    .iter()
                                    .any(|(k, _)| k.0 == id && k.1 == p.panel);
                            if known {
                                let key = (id, p.panel);
                                self.plugins.set_view(key.clone(), p.root);
                                follow = self.plugin_grids_follow(&key);
                            }
                        }
                    }
                    methods::NOTIFY => {
                        if let Ok(p) = serde_json::from_value::<NotifyParams>(params) {
                            self.set_status(p.message, p.level == Level::Error);
                        }
                    }
                    methods::SEARCH_ITEMS => {
                        if let Ok(p) = serde_json::from_value::<SearchItemsParams>(params) {
                            self.plugins.search_items.insert(id, p.items);
                            if self.overlay == super::ActiveOverlay::Everywhere {
                                self.everywhere_refresh();
                            }
                        }
                    }
                    methods::DECORATE => {
                        if let Ok(p) = serde_json::from_value::<DecorateParams>(params) {
                            self.plugins.decorations.insert(id, p.files);
                            self.plugins_decorate();
                        }
                    }
                    methods::SET_BASE => {
                        if let Ok(p) = serde_json::from_value::<SetBaseParams>(params) {
                            let base = if p.none {
                                crate::plugins::Base::None
                            } else {
                                crate::plugins::Base::Text(p.text)
                            };
                            self.plugins.bases.insert(p.path.clone(), base.clone());
                            if self.current_file.as_ref() == Some(&p.path) {
                                self.plugin_base_loaded(p.path, base);
                            }
                        }
                    }
                    methods::SET_ORIGIN => {
                        if let Ok(p) = serde_json::from_value::<
                            ardoise_plugin::protocol::SetOriginParams,
                        >(params)
                        {
                            match p.origin {
                                Some(origin) => self.plugins.origins.insert(p.path, origin),
                                None => self.plugins.origins.remove(&p.path),
                            };
                        }
                    }
                    methods::ANNOTATE => {
                        if let Ok(p) = serde_json::from_value::<AnnotateParams>(params) {
                            self.plugin_annotate(id, p);
                        }
                    }
                    methods::COPY => {
                        if let Ok(p) = serde_json::from_value::<CopyParams>(params) {
                            return iced::clipboard::write(p.text);
                        }
                    }
                    methods::TAB_DIRTY => {
                        if let Ok(p) = serde_json::from_value::<TabDirtyParams>(params) {
                            if let Some(tab) = self
                                .plugins
                                .tabs
                                .values_mut()
                                .find(|t| t.plugin == id && t.tab == p.tab)
                            {
                                tab.dirty = p.dirty;
                            }
                        }
                    }
                    methods::CLOSE_TAB => {
                        if let Ok(p) = serde_json::from_value::<PanelParams>(params) {
                            // A dialog the plugin closed itself: it is not
                            // told again that it was closed.
                            let key: PanelKey = (id.clone(), p.panel.clone());
                            if let Some(at) =
                                self.plugins.dialogs.iter().position(|(k, _)| *k == key)
                            {
                                self.plugins.dialogs.remove(at);
                                self.plugins.views.remove(&key);
                                self.plugins.revealed.retain(|(k, _)| *k != key);
                                return Task::none();
                            }
                            let path = self
                                .plugins
                                .tabs
                                .iter()
                                .find(|(_, t)| t.plugin == id && t.tab == p.panel)
                                .map(|(path, _)| path.clone());
                            if let Some(path) = path {
                                return self.close_path(path);
                            }
                        }
                    }
                    methods::SHOW_PANEL => {
                        if let Ok(p) = serde_json::from_value::<PanelParams>(params) {
                            let key = (id, p.panel);
                            let open = [&self.plugins.sidebar, &self.plugins.bottom]
                                .into_iter()
                                .any(|k| k.as_ref() == Some(&key));
                            if !open {
                                self.toggle_plugin_panel(key);
                            }
                        }
                    }
                    methods::PUBLISH_DIAGNOSTICS => {
                        if let Ok(p) = serde_json::from_value::<PublishParams>(params) {
                            self.plugin_published(id, p);
                        }
                    }
                    methods::STATUS => {
                        if let Ok(p) = serde_json::from_value::<StatusParams>(params) {
                            let key = (id, p.id.clone());
                            if p.text.is_some() {
                                self.plugins.status.insert(key, p);
                            } else {
                                self.plugins.status.remove(&key);
                            }
                        }
                    }
                    _ => {}
                }
                follow
            }
            Rpc::Request {
                id: request,
                method,
                params,
            } => match method.as_str() {
                methods::CONFIRM => match serde_json::from_value::<ConfirmParams>(params) {
                    Ok(p) => {
                        // A question already asked is answered no first.
                        if let ActiveOverlay::PluginConfirm(old) = &self.overlay {
                            host::respond(&old.plugin, old.request, Ok(Value::Bool(false)));
                        }
                        self.overlay = ActiveOverlay::PluginConfirm(Box::new(PluginQuestion {
                            plugin: id,
                            request,
                            message: p.message,
                            confirm: p.confirm,
                        }));
                        Task::none()
                    }
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::OPEN_FILE => match serde_json::from_value::<OpenFileParams>(params) {
                    Ok(p) => {
                        host::respond(&id, request, Ok(Value::Null));
                        self.jump_to(crate::definition::Hit {
                            path: p.path,
                            line: p.line.unwrap_or(1).saturating_sub(1),
                            col: 0,
                            len: 0,
                        })
                    }
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::PICK_SAVE_PATH => {
                    match serde_json::from_value::<rpc::PickSavePathParams>(params) {
                        Ok(p) => Task::perform(
                            async move {
                                let picked = rfd::AsyncFileDialog::new()
                                    .set_file_name(p.name)
                                    .save_file()
                                    .await;
                                Ok(picked.map_or(Value::Null, |f| {
                                    Value::String(f.path().to_string_lossy().into_owned())
                                }))
                            },
                            move |result| {
                                plugin_msg(PluginMsg::Respond(id.clone(), request, result))
                            },
                        ),
                        Err(e) => {
                            host::respond(&id, request, Err(invalid(e)));
                            Task::none()
                        }
                    }
                }
                methods::PICK_FILE => match serde_json::from_value::<PickFileParams>(params) {
                    Ok(p) => Task::perform(
                        async move {
                            let mut dialog = rfd::AsyncFileDialog::new();
                            if !p.title.is_empty() {
                                dialog = dialog.set_title(p.title);
                            }
                            if !p.extensions.is_empty() {
                                dialog = dialog.add_filter("", &p.extensions);
                            }
                            let picked = dialog.pick_file().await;
                            Ok(picked.map_or(Value::Null, |f| {
                                Value::String(f.path().to_string_lossy().into_owned())
                            }))
                        },
                        move |result| plugin_msg(PluginMsg::Respond(id.clone(), request, result)),
                    ),
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::OPEN_MERGE => match serde_json::from_value::<MergeParams>(params) {
                    Ok(p) => match self.plugin_merge(id.clone(), p) {
                        Some(task) => {
                            host::respond(&id, request, Ok(Value::Null));
                            task
                        }
                        None => {
                            let error = invalid(t!("plugins.no_conflict"));
                            host::respond(&id, request, Err(error));
                            Task::none()
                        }
                    },
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::OPEN_DIFF_TAB => match serde_json::from_value::<DiffTabParams>(params) {
                    Ok(p) => {
                        host::respond(&id, request, Ok(Value::Null));
                        self.plugin_diff_tab(id, p)
                    }
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::OPEN_TAB => match serde_json::from_value::<TabParams>(params) {
                    Ok(p) => match self.plugin_open_tab(&id, p) {
                        Ok(task) => {
                            host::respond(&id, request, Ok(Value::Null));
                            task
                        }
                        Err(e) => {
                            host::respond(&id, request, Err(invalid(e)));
                            Task::none()
                        }
                    },
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::SAVE_FILE => match serde_json::from_value::<SaveFileParams>(params) {
                    Ok(p) => {
                        let title = self.plugins.get(&id).map(|x| x.name()).unwrap_or_default();
                        Task::perform(
                            async move {
                                let picked = rfd::AsyncFileDialog::new()
                                    .set_title(title)
                                    .set_file_name(p.name)
                                    .save_file()
                                    .await;
                                let Some(handle) = picked else {
                                    return Ok(Value::Bool(false));
                                };
                                let bytes = p.bytes.unwrap_or_else(|| p.text.into_bytes());
                                tokio::fs::write(handle.path(), bytes)
                                    .await
                                    .map(|()| Value::Bool(true))
                                    .map_err(|e| RpcError {
                                        code: codes::INTERNAL_ERROR,
                                        message: e.to_string(),
                                    })
                            },
                            move |result| {
                                plugin_msg(PluginMsg::Respond(id.clone(), request, result))
                            },
                        )
                    }
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::OPEN_DIFF => match serde_json::from_value::<DiffParams>(params) {
                    Ok(p) => {
                        host::respond(&id, request, Ok(Value::Null));
                        self.open_comparison(
                            (p.left.title, p.left.text),
                            (p.right.title, p.right.text),
                        )
                    }
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::REPLACE_TEXT => {
                    match serde_json::from_value::<ReplaceTextParams>(params) {
                        Ok(p) if self.current_file.as_ref() == Some(&p.path) => {
                            if self.editing_blocked() {
                                host::respond(
                                    &id,
                                    request,
                                    Err(invalid(t!("plugins.err_read_only"))),
                                );
                                return Task::none();
                            }
                            host::respond(&id, request, Ok(Value::Null));
                            let (line, col) = self.editor.content.cursor_position();
                            self.apply_text_edit(p.text, line, col, None)
                        }
                        Ok(_) => {
                            host::respond(
                                &id,
                                request,
                                Err(invalid(t!("plugins.err_not_in_front"))),
                            );
                            Task::none()
                        }
                        Err(e) => {
                            host::respond(&id, request, Err(invalid(e)));
                            Task::none()
                        }
                    }
                }
                methods::EDITOR_STATE => {
                    let answer = serde_json::from_value::<rpc::EditorStateParams>(params)
                        .map_err(invalid)
                        .and_then(|p| {
                            let slot = ((id.clone(), p.panel), p.editor);
                            let (content, _) = self
                                .plugins
                                .editors
                                .get(&slot)
                                .ok_or_else(|| invalid("no such editor"))?;
                            let at =
                                |p: iced::advanced::text::editor::Position| rpc::TextPosition {
                                    line: p.line,
                                    column: p.column,
                                };
                            let cursor = content.cursor();
                            serde_json::to_value(rpc::EditorState {
                                text: content.text(),
                                cursor: at(cursor.position),
                                anchor: cursor.selection.map(at),
                            })
                            .map_err(invalid)
                        });
                    host::respond(&id, request, answer);
                    Task::none()
                }
                methods::OPEN_TEXT => match serde_json::from_value::<OpenTextParams>(params) {
                    Ok(p) => {
                        let path = self.plugin_open_text(p);
                        host::respond(&id, request, serde_json::to_value(path).map_err(invalid));
                        Task::none()
                    }
                    Err(e) => {
                        host::respond(&id, request, Err(invalid(e)));
                        Task::none()
                    }
                },
                methods::ACTIVE_FILE => {
                    // The line pointed at in the gutter, or the selection.
                    let pointed = self.plugins.pointed_line.take();
                    let lines = self.current_file.as_ref().map(|_| match pointed {
                        Some(line) => (line + 1, line + 1),
                        None => {
                            let (first, last) = self.lines_to_edit(self.selection_span());
                            (first + 1, last + 1)
                        }
                    });
                    let active = ActiveFile {
                        path: self.plugin_context().file,
                        text: crate::editor::buffer_text(&self.editor.content),
                        selection: self.editor.content.selection(),
                        language: self.editor.language.clone(),
                        lines,
                    };
                    host::respond(&id, request, serde_json::to_value(active).map_err(invalid));
                    Task::none()
                }
                _ => {
                    host::respond(
                        &id,
                        request,
                        Err(RpcError {
                            code: codes::METHOD_NOT_FOUND,
                            message: method,
                        }),
                    );
                    Task::none()
                }
            },
            Rpc::Response { .. } => Task::none(),
        }
    }

    // ─── What the plugins add to the window ──────────────────────────────

    /// The buttons of the panels of the plugins at `location`, for the
    /// activity bar: icon, open, tooltip, message.
    /// The settings of the Lint plugin opened, for its page of the
    /// preferences.
    pub(super) fn plugin_lint_settings_open(&mut self) -> Task<Message> {
        match self.plugins.linter().map(|l| l.id().to_string()) {
            Some(id) => self.plugin_settings_show(&id),
            None => Task::none(),
        }
    }

    /// The settings of a plugin opened (the one open before closes), for its
    /// page of the preferences; they stay open if they are.
    pub(super) fn plugin_settings_show(&mut self, id: &str) -> Task<Message> {
        let Some(panel) = self.plugins.settings_panel(id).map(|p| p.id.clone()) else {
            return Task::none();
        };
        if self.plugins.settings_open.as_ref() == Some(&(id.to_string(), panel)) {
            return Task::none();
        }
        self.plugin_update(PluginMsg::ToggleSettings(id.to_string()))
    }

    pub(super) fn plugin_tools(&self, location: Location) -> Vec<(String, bool, String, Message)> {
        let open = match location {
            Location::Sidebar => self.plugins.sidebar.as_ref().filter(|_| self.show_sidebar),
            Location::Bottom => self.plugins.bottom.as_ref(),
            // No button: the settings are in the preferences.
            Location::Settings => return Vec::new(),
        };
        let locale = rust_i18n::locale();
        self.plugins
            .panels(location)
            .into_iter()
            .map(|(plugin, panel)| {
                let title = panel.title.get(&locale).to_string();
                let icon = panel_icon(panel.icon.as_deref(), &title);
                let is_open = open.is_some_and(|k| k.0 == plugin.id() && k.1 == panel.id);
                (
                    icon,
                    is_open,
                    title,
                    plugin_msg(PluginMsg::TogglePanel(
                        plugin.id().to_string(),
                        panel.id.clone(),
                    )),
                )
            })
            .collect()
    }

    /// The content of a panel: what the plugin sent, or why there is nothing.
    fn plugin_panel_body<'a>(&'a self, key: &PanelKey, dark: bool) -> Element<'a, Message> {
        let p = se_theme::palette(dark);
        if let Some(node) = self.plugins.views.get(key) {
            return crate::plugins::render::view(&self.plugins, key, node, dark);
        }
        if let Some(why) = self.plugins.crashed.get(&key.0) {
            return iced::widget::column![
                text(t!("plugins.panel_stopped").to_string())
                    .size(13)
                    .color(p.text),
                text(why.clone())
                    .size(12)
                    .color(p.muted)
                    .font(se_theme::MONO),
                button(text(t!("plugins.restart").to_string()).size(12.5))
                    .padding([5, 12])
                    .on_press(plugin_msg(PluginMsg::Restart(key.0.clone())))
                    .style((se_theme::Outline { dark }).style()),
            ]
            .spacing(8)
            .into();
        }
        text(t!("plugins.panel_loading").to_string())
            .size(12)
            .color(p.muted)
            .into()
    }

    fn plugin_panel_header<'a>(
        &'a self,
        key: &PanelKey,
        dark: bool,
        close: Option<Message>,
    ) -> Element<'a, Message> {
        let p = se_theme::palette(dark);
        let mut header = row![text(self.plugins.panel_title(key).to_uppercase())
            .size(10.5)
            .font(se_theme::UI_MEDIUM)
            .color(p.muted)]
        .align_y(Alignment::Center);
        if let Some(close) = close {
            header = header.push(Space::new().width(Length::Fill)).push(
                button(text("×").size(16).color(p.muted))
                    .padding([2, 6])
                    .on_press(close)
                    .style(
                        (se_theme::GhostButton {
                            dark,
                            active: false,
                        })
                        .style(),
                    ),
            );
        }
        header.into()
    }

    /// The panel of a plugin in the sidebar, if one is open.
    pub(super) fn plugin_sidebar(&self, dark: bool) -> Option<Element<'_, Message>> {
        let key = self.plugins.sidebar.as_ref()?;
        Some(
            container(
                iced::widget::column![
                    self.plugin_panel_header(key, dark, None),
                    self.plugin_panel_body(key, dark)
                ]
                .spacing(10),
            )
            .padding([10, 12])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(se_theme::panel(dark))
            .into(),
        )
    }

    /// The panel of a plugin under the editor, if one is open.
    pub(super) fn plugin_bottom(&self, dark: bool) -> Option<Element<'_, Message>> {
        let key = self.plugins.bottom.as_ref()?;
        Some(
            container(iced::widget::column![
                rule(dark),
                container(
                    iced::widget::column![
                        self.plugin_panel_header(
                            key,
                            dark,
                            Some(plugin_msg(PluginMsg::CloseBottom))
                        ),
                        self.plugin_panel_body(key, dark)
                    ]
                    .spacing(6)
                )
                .padding([6, 14])
                .height(Length::Fill)
            ])
            .height(Length::Fixed(self.panel_height))
            .width(Length::Fill)
            .into(),
        )
    }

    /// The items of the status bar the plugins set: text, tooltip, click.
    pub(super) fn plugin_status_items(&self) -> Vec<(String, Option<String>, Option<Message>)> {
        self.plugins
            .status
            .iter()
            .map(|((plugin, _), item)| {
                (
                    item.text.clone().unwrap_or_default(),
                    item.tooltip.clone(),
                    item.command
                        .clone()
                        .map(|c| plugin_msg(PluginMsg::Run(plugin.clone(), c))),
                )
            })
            .collect()
    }

    /// The commands of the plugins in the context menus.
    pub(super) fn plugin_menus(&self) -> Arc<Vec<crate::plugins::MenuItem>> {
        let locale = rust_i18n::locale().to_string();
        let mut items = Vec::new();
        for plugin in &self.plugins.installed {
            for command in &plugin.manifest.contributes.commands {
                for menu in &command.menus {
                    items.push(crate::plugins::MenuItem {
                        menu: *menu,
                        plugin: plugin.id().to_string(),
                        command: command.id.clone(),
                        label: command.title.get(&locale).to_string(),
                    });
                }
            }
        }
        Arc::new(items)
    }

    /// The commands of the plugins, for the search for actions: (plugin,
    /// command, label).
    pub(super) fn plugin_commands(&self) -> Vec<(String, String, String)> {
        let locale = rust_i18n::locale().to_string();
        self.plugins
            .installed
            .iter()
            .flat_map(|p| {
                let name = p.name();
                let locale = locale.clone();
                p.manifest.contributes.commands.iter().map(move |c| {
                    (
                        p.id().to_string(),
                        c.id.clone(),
                        format!("{}: {}", name, c.title.get(&locale)),
                    )
                })
            })
            .collect()
    }

    pub(super) fn view_plugin_question(
        &self,
        dark: bool,
        q: &PluginQuestion,
    ) -> Element<'static, Message> {
        let p = se_theme::palette(dark);
        let name = self
            .plugins
            .get(&q.plugin)
            .map(|x| x.name())
            .unwrap_or(q.plugin.clone());
        let no = plugin_msg(PluginMsg::Answer(q.plugin.clone(), q.request, false));
        let yes = plugin_msg(PluginMsg::Answer(q.plugin.clone(), q.request, true));
        use crate::popin::{action, Action};
        let body = text(q.message.clone())
            .size(13)
            .color(p.muted)
            .shaping(iced::widget::text::Shaping::Advanced);
        crate::popin::card(
            dark,
            crate::popin::WIDTH_CONFIRM,
            name,
            Some(no.clone()),
            body.into(),
            Some((
                vec![],
                vec![
                    action(
                        dark,
                        t!("confirm.cancel").to_string(),
                        Some(no),
                        Action::Cancel,
                    ),
                    action(
                        dark,
                        q.confirm
                            .clone()
                            .unwrap_or_else(|| t!("plugins.ok").to_string()),
                        Some(yes),
                        Action::Primary,
                    ),
                ],
            )),
        )
    }

    /// Escape or a click beside the question: it is answered no.
    pub(super) fn dismiss_plugin_question(&mut self) {
        if let ActiveOverlay::PluginConfirm(q) = &self.overlay {
            host::respond(&q.plugin, q.request, Ok(Value::Bool(false)));
        }
    }
}

/// Whether the editor `id` of a tree of nodes is only read.
fn editor_read_only(node: &ardoise_plugin::ui::Node, id: &str) -> bool {
    use ardoise_plugin::ui::Node;
    match node {
        Node::Editor {
            id: this,
            read_only,
            ..
        } => this == id && *read_only,
        Node::Column { children, .. } | Node::Row { children, .. } => {
            children.iter().any(|c| editor_read_only(c, id))
        }
        Node::Scroll { child, .. } => editor_read_only(child, id),
        _ => false,
    }
}

/// The files under `dir` (not in hidden folders), at most `max`.
fn walk_files(dir: &Path, max: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let hidden = entry.file_name().to_string_lossy().starts_with('.');
            match entry.file_type() {
                Ok(t) if t.is_dir() && !hidden => stack.push(path),
                Ok(t) if t.is_file() => out.push(path),
                _ => {}
            }
            if out.len() >= max {
                return out;
            }
        }
    }
    out
}

fn invalid(e: impl std::fmt::Display) -> RpcError {
    RpcError {
        code: codes::INVALID_PARAMS,
        message: e.to_string(),
    }
}

/// The icon of a panel in the activity bar: its glyph, or the first letter
/// of its title when it has none (an empty one would be a blank square).
fn panel_icon(icon: Option<&str>, title: &str) -> String {
    match icon.map(str::trim).filter(|i| !i.is_empty()) {
        Some(icon) => icon.to_string(),
        None => title
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod icon_tests {
    use super::panel_icon;

    #[test]
    fn a_panel_without_a_glyph_shows_the_letter_of_its_title() {
        assert_eq!(panel_icon(Some("\u{e725}"), "Git"), "\u{e725}");
        assert_eq!(panel_icon(Some(""), "git"), "G");
        assert_eq!(panel_icon(Some("  "), "base"), "B");
        assert_eq!(panel_icon(None, "Évents"), "É");
        assert_eq!(panel_icon(None, ""), "");
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// A plugin in Python: it answers `initialize`, shows "hi" in its panel,
    /// sets a status item and asks a question on its command, says what it
    /// was answered, and dies when its "die" button is pressed.
    const PLUGIN: &str = r#"
import sys, json
def read():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        line = line.strip()
        if not line:
            if headers:
                break
            continue
        k, v = line.decode().split(':', 1)
        headers[k.lower()] = v.strip()
    return json.loads(sys.stdin.buffer.read(int(headers['content-length'])))
def send(m):
    m['jsonrpc'] = '2.0'
    b = json.dumps(m).encode()
    sys.stdout.buffer.write(b'Content-Length: %d\r\n\r\n' % len(b) + b)
    sys.stdout.buffer.flush()
while True:
    m = read()
    if m is None:
        break
    method = m.get('method')
    if method == 'initialize':
        send({'id': m['id'], 'result': None})
    elif method == 'shutdown':
        send({'id': m['id'], 'result': None})
        break
    elif method == 'panel/opened':
        send({'method': 'ui/render', 'params': {'panel': m['params']['panel'], 'root': {'type': 'text', 'text': 'hi'}}})
    elif method == 'command':
        send({'method': 'statusBar/set', 'params': {'id': 's', 'text': 'ready'}})
        send({'id': 7, 'method': 'window/confirm', 'params': {'message': 'Sure?'}})
    elif method is None and 'id' in m:
        send({'method': 'window/notify', 'params': {'message': 'answer %s' % json.dumps(m.get('result'))}})
    elif method == 'lint/run':
        n = len(m['params']['text'])
        send({'id': m['id'], 'result': {'diagnostics': [{'line': 0, 'col': 0, 'end_line': 0, 'end_col': n,
              'severity': 'error', 'message': 'bad', 'source': 'Fake'}], 'ran': ['Fake']}})
    elif method == 'ui/event' and m['params']['action'] == 'die':
        sys.exit(3)
"#;

    fn next(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Incoming>) -> Incoming {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Ok(incoming) = rx.try_recv() {
                return incoming;
            }
            assert!(Instant::now() < deadline, "the plugin said nothing");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_plugin_runs_shows_a_panel_asks_and_stops() {
        if crate::lsp::find_executable("python3").is_none() {
            return;
        }
        let Some(mut rx) = host::take_events() else {
            panic!("the events are listened to elsewhere");
        };
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("plugin.py"), PLUGIN).unwrap();
        std::fs::write(
            folder.path().join("plugin.json"),
            r#"{"id": "test.e2e", "name": "E2E", "version": "0.1.0", "api": 1,
                "command": ["python3", "plugin.py"],
                "contributes": {
                    "commands": [{"id": "e2e.ask", "title": "Ask"}],
                    "panels": [{"id": "main", "title": "Main"}]
                }}"#,
        )
        .unwrap();
        store::link_folder(folder.path()).unwrap();

        let mut app = ArdoiseApp::blank(Config::default());
        app.plugins.reload();
        let id = "test.e2e".to_string();
        assert!(app
            .plugin_commands()
            .iter()
            .any(|(p, c, label)| p == &id && c == "e2e.ask" && label == "E2E: Ask"));
        assert_eq!(app.plugin_tools(Location::Sidebar).len(), 1);

        // The panel opens: the plugin starts and draws it.
        let _ = app.update(Message::Plugin(PluginMsg::TogglePanel(
            id.clone(),
            "main".into(),
        )));
        assert!(host::is_running(&id));
        assert_eq!(app.plugins.sidebar, Some((id.clone(), "main".into())));
        let incoming = next(&mut rx);
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(incoming)));
        assert_eq!(
            app.plugins.views.get(&(id.clone(), "main".into())),
            Some(&ardoise_plugin::ui::text("hi"))
        );

        // Its command sets a status item and asks a question.
        let _ = app.update(Message::Plugin(PluginMsg::Run(
            id.clone(),
            "e2e.ask".into(),
        )));
        for _ in 0..2 {
            let incoming = next(&mut rx);
            let _ = app.update(Message::Plugin(PluginMsg::Incoming(incoming)));
        }
        assert_eq!(app.plugin_status_items()[0].0, "ready");
        let ActiveOverlay::PluginConfirm(question) = &app.overlay else {
            panic!("no question");
        };
        assert_eq!((question.request, question.message.as_str()), (7, "Sure?"));
        let _ = app.update(Message::Plugin(PluginMsg::Answer(id.clone(), 7, true)));
        assert!(matches!(app.overlay, ActiveOverlay::None));
        let incoming = next(&mut rx);
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(incoming)));
        assert_eq!(app.status_message, "answer true");

        // The editor asks, and the answer comes back to whoever waits.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let params = serde_json::json!({"path": "/x/a.json", "text": "abc"});
        let answer = runtime
            .block_on(host::request(
                &id,
                methods::LINT_RUN,
                &params,
                Duration::from_secs(20),
            ))
            .unwrap();
        let result: ardoise_plugin::lint::LintResult = serde_json::from_value(answer).unwrap();
        assert_eq!(result.diagnostics[0].end_col, 3);
        assert_eq!(result.ran, vec!["Fake".to_string()]);
        // A question it never answers is given up on.
        let silent = runtime.block_on(host::request(
            &id,
            "lint/nothing",
            Value::Null,
            Duration::from_millis(300),
        ));
        assert!(silent.is_err());

        // It dies: what it showed goes, the panel offers to start it again.
        let _ = app.update(Message::Plugin(PluginMsg::Ui(
            id.clone(),
            "main".into(),
            "die".into(),
            None,
        )));
        let incoming = next(&mut rx);
        assert!(matches!(&incoming, Incoming::Crashed(who, _) if who == &id));
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(incoming)));
        assert!(app.plugins.crashed.contains_key(&id));
        assert!(app.plugins.views.is_empty());
        assert!(app.plugin_status_items().is_empty());
        assert!(!host::is_running(&id));

        // Uninstalled, it is gone from the window.
        let _ = app.update(Message::Plugin(PluginMsg::Uninstall(id.clone())));
        assert!(app.plugins.get(&id).is_none());
        assert_eq!(app.plugins.sidebar, None);
        assert!(folder.path().join("plugin.json").is_file());
    }

    #[test]
    fn a_question_closed_without_an_answer_is_a_no() {
        let mut app = ArdoiseApp::blank(Config::default());
        app.overlay = ActiveOverlay::PluginConfirm(Box::new(PluginQuestion {
            plugin: "test.none".into(),
            request: 1,
            message: "?".into(),
            confirm: None,
        }));
        let _ = app.update(Message::CloseOverlay);
        assert!(matches!(app.overlay, ActiveOverlay::None));
    }
}

/// A conflict of a plugin was resolved: the file is written, the plugin told.
pub(super) async fn merge_written(
    plugin: String,
    tab: String,
    file: PathBuf,
    text: String,
) -> Result<(), String> {
    tokio::fs::write(&file, &text)
        .await
        .map_err(|e| e.to_string())?;
    host::notify(
        &plugin,
        methods::MERGE_RESOLVED,
        MergeParams {
            tab,
            path: file,
            text,
            swap: false,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tab_tests {
    use super::*;
    use ardoise_plugin::ui::{column, editor, text};

    fn app_with_plugin() -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "test.tabs", "name": "Tabs", "version": "1.0.0", "api": 1, "command": ["bin/none"]}"#,
        )
        .unwrap();
        app.plugins.installed.push(Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
        app
    }

    fn from_plugin(app: &mut ArdoiseApp, message: Rpc) {
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(Incoming::Rpc(
            "test.tabs".into(),
            message,
        ))));
    }

    fn open_dialog_request(tab: &str, title: &str) -> Rpc {
        Rpc::request(
            9,
            methods::OPEN_TAB,
            serde_json::json!({"tab": tab, "title": title, "dialog": true}),
        )
    }

    #[test]
    fn a_plugin_opens_a_dialog_in_place_of_a_tab_and_it_closes_by_the_cross_or_escape() {
        let mut app = app_with_plugin();
        let key: PanelKey = ("test.tabs".into(), "form".into());
        from_plugin(&mut app, open_dialog_request("form", "New source"));
        assert_eq!(
            app.plugins.dialogs,
            vec![(key.clone(), "New source".into())]
        );
        assert!(app.plugins.tabs.is_empty(), "no tab among the files");
        assert!(app.current_file.is_none() || !is_plugin_tab(app.current_file.as_ref().unwrap()));
        // Opened again: in front, not twice.
        from_plugin(&mut app, open_dialog_request("other", "Other"));
        from_plugin(&mut app, open_dialog_request("form", "New source"));
        assert_eq!(app.plugins.dialogs.len(), 2);
        assert_eq!(app.plugin_front_dialog(), Some(&key));
        // It renders, and the cross closes it.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::RENDER,
                RenderParams {
                    panel: "form".into(),
                    root: column([text("hello")]),
                },
            ),
        );
        assert!(app.plugins.views.contains_key(&key));
        let _ = app.update(Message::Plugin(PluginMsg::CloseDialog(
            "test.tabs".into(),
            "form".into(),
        )));
        assert_eq!(app.plugins.dialogs.len(), 1);
        assert!(
            !app.plugins.views.contains_key(&key),
            "its view is forgotten"
        );
        // Escape closes the one left.
        let _ = app.update(Message::EscapePressed);
        assert!(app.plugins.dialogs.is_empty());
    }

    #[test]
    fn a_dialog_the_plugin_closes_goes_without_a_tab_and_stays_out_of_the_session() {
        let mut app = app_with_plugin();
        from_plugin(&mut app, open_dialog_request("form", "New source"));
        assert!(app.plugins.tabs.is_empty(), "the session keeps tabs only");
        from_plugin(
            &mut app,
            Rpc::notification(methods::CLOSE_TAB, serde_json::json!({"panel": "form"})),
        );
        assert!(app.plugins.dialogs.is_empty());
    }

    #[test]
    fn a_secret_field_is_shown_in_clear_and_hidden_again_by_its_eye() {
        let mut app = app_with_plugin();
        let slot = (
            ("test.tabs".to_string(), "form".to_string()),
            "pw".to_string(),
        );
        let eye = || {
            Message::Plugin(PluginMsg::Reveal(
                "test.tabs".into(),
                "form".into(),
                "pw".into(),
            ))
        };
        let _ = app.update(eye());
        assert!(app.plugins.revealed.contains(&slot));
        let _ = app.update(eye());
        assert!(!app.plugins.revealed.contains(&slot));
    }

    #[test]
    fn a_plugin_opens_a_tab_shows_an_editor_in_it_and_keeps_it_in_the_session() {
        let mut app = app_with_plugin();
        from_plugin(
            &mut app,
            Rpc::request(
                4,
                methods::OPEN_TAB,
                serde_json::json!({"tab": "req-1", "title": "GET /items"}),
            ),
        );
        let path = crate::plugins::tab_path("test.tabs", "req-1", "GET /items");
        assert_eq!(app.current_file.as_ref(), Some(&path));
        assert!(app.sidebar.files().contains(&path));
        assert_eq!(display_name(&path), "GET /items");
        assert!(is_virtual(&path) && is_untitled(&path));
        // What it renders for the tab, with an editor.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::RENDER,
                RenderParams {
                    panel: "req-1".into(),
                    root: column([editor("body", "{}", "json")]),
                },
            ),
        );
        let slot = (
            ("test.tabs".to_string(), "req-1".to_string()),
            "body".to_string(),
        );
        assert_eq!(app.plugins.editors[&slot].0.text(), "{}");
        let _ = app.update(Message::Plugin(PluginMsg::EditorAction(
            "test.tabs".into(),
            "req-1".into(),
            "body".into(),
            text_editor::Action::Edit(text_editor::Edit::Insert('x')),
        )));
        assert_eq!(app.plugins.editors[&slot].0.text(), "x{}");
        // A render with the same value keeps what was typed.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::RENDER,
                RenderParams {
                    panel: "req-1".into(),
                    root: column([editor("body", "{}", "json")]),
                },
            ),
        );
        assert_eq!(app.plugins.editors[&slot].0.text(), "x{}");
        // Changes not kept: the tab says so.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::TAB_DIRTY,
                serde_json::json!({"tab": "req-1", "dirty": true}),
            ),
        );
        assert!(app.is_path_dirty(&path));
        // The same tab again with a new title is renamed, not doubled.
        from_plugin(
            &mut app,
            Rpc::request(
                5,
                methods::OPEN_TAB,
                serde_json::json!({"tab": "req-1", "title": "POST /items"}),
            ),
        );
        let renamed = crate::plugins::tab_path("test.tabs", "req-1", "POST /items");
        assert_eq!(app.plugins.tabs.len(), 1);
        assert_eq!(app.current_file.as_ref(), Some(&renamed));
        let session = app.build_session();
        assert_eq!(
            session.plugin_tabs,
            vec![renamed.to_string_lossy().into_owned()]
        );
        // A bad id is refused.
        from_plugin(
            &mut app,
            Rpc::request(
                6,
                methods::OPEN_TAB,
                serde_json::json!({"tab": "a/b", "title": "x"}),
            ),
        );
        assert_eq!(app.plugins.tabs.len(), 1);
        // The plugin closes it.
        from_plugin(
            &mut app,
            Rpc::notification(methods::CLOSE_TAB, serde_json::json!({"panel": "req-1"})),
        );
        assert!(app.plugins.tabs.is_empty());
        assert!(!app.sidebar.files().contains(&renamed));
    }

    #[test]
    fn a_plugin_colors_files_marks_changed_lines_and_annotates_them() {
        let mut app = app_with_plugin();
        let file = PathBuf::from("/repo/src/a.rs");
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: file.clone(),
            text: "one\nTWO\nthree\nfour\n".into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        // How files stand: in the tree and on the tabs.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::DECORATE,
                serde_json::json!({"files": [{"path": "/repo/src/a.rs", "state": "modified"}]}),
            ),
        );
        assert_eq!(
            app.file_marks.git(&file, false),
            Some(crate::diff::FileState::Modified)
        );
        assert!(
            app.file_marks.git(Path::new("/repo/src"), true).is_some(),
            "its folder too"
        );
        // What the file is compared with: its changed lines are marked.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::SET_BASE,
                serde_json::json!({"path": "/repo/src/a.rs", "text": "one\ntwo\nthree\n"}),
            ),
        );
        let marks = app.marks.as_ref().expect("marks");
        assert_eq!(marks.marks.modified, vec![1]);
        assert_eq!(marks.marks.added, vec![3]);
        // A column of annotations, and a click on it goes to the plugin.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::ANNOTATE,
                serde_json::json!({"path": "/repo/src/a.rs", "annotations": {
                    "lines": [{"text": "ada 2026"}, {"text": "bob 2025", "tint": [1, 2, 3], "detail": 0}],
                    "details": [["abc123", "fix"]]
                }}),
            ),
        );
        let blame = app.blame.as_ref().expect("a column");
        assert_eq!(blame.annotations.lines[1].text, "bob 2025");
        assert!(app.plugin_annotation_clicked(1));
        // The plugin takes it away.
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::ANNOTATE,
                serde_json::json!({"path": "/repo/src/a.rs"}),
            ),
        );
        assert!(app.blame.is_none());
        assert!(!app.plugin_annotation_clicked(1));
    }

    #[test]
    fn a_plugin_shows_its_diffs_in_a_tab_and_replaces_them_in_place() {
        let mut app = app_with_plugin();
        let patch =
            "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,2 @@\n one\n-two\n+TWO\n";
        let open = |app: &mut ArdoiseApp, patch: &str| {
            from_plugin(
                app,
                Rpc::request(
                    7,
                    methods::OPEN_DIFF_TAB,
                    serde_json::json!({"tab": "changes", "title": "Changes", "files": [
                        {"path": "a.rs", "patch": patch, "mode": "unstaged", "file": "/repo/a.rs"},
                        {"path": "b.rs", "patch": "", "mode": "plain"}
                    ]}),
                ),
            );
        };
        open(&mut app, patch);
        let path = app.current_file.clone().expect("a tab in front");
        let tab = app.diff_tabs.get(&path).expect("a diff tab");
        assert_eq!(tab.viewer.entries.len(), 2);
        let diff = tab.viewer.file_diff().expect("its diff, given");
        assert_eq!(diff.path, "a.rs");
        assert_eq!(tab.viewer.changes(), 1);
        assert!(tab
            .owner
            .as_ref()
            .is_some_and(|(p, t, _)| p == "test.tabs" && t == "changes"));
        // Its buttons go to the plugin: Git is not asked.
        let _ = app.update(Message::DiffTab(DiffNav::ToggleBlanks));
        assert!(app.diff_tabs[&path].viewer.ignore_blanks);
        // The same tab id: what it shows is replaced, no new tab.
        let tabs = app.diff_tabs.len();
        open(&mut app, &patch.replace("TWO", "2"));
        assert_eq!(app.diff_tabs.len(), tabs);
        let tab = &app.diff_tabs[&path];
        assert!(format!("{:?}", tab.viewer.file_diff()).contains('2'));
        // Moving to the next file reads the given diff, not Git.
        let _ = app.update(Message::DiffTab(DiffNav::NextFile));
        assert_eq!(app.diff_tabs[&path].viewer.index, Some(1));
        assert!(matches!(app.diff_tabs[&path].viewer.diff, Some(Ok(_))));
    }

    #[test]
    fn a_plugin_gives_a_conflict_that_is_resolved_in_a_tab_and_written() {
        let mut app = app_with_plugin();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        let text = "top\n<<<<<<< ours\nmine\n=======\ntheirs\n>>>>>>> other\nend\n";
        std::fs::write(&file, text).unwrap();
        // No conflict in it: refused.
        from_plugin(
            &mut app,
            Rpc::request(
                8,
                methods::OPEN_MERGE,
                serde_json::json!({"tab": "m", "path": file, "text": "plain\n"}),
            ),
        );
        assert!(app.merge_tabs.is_empty());
        from_plugin(
            &mut app,
            Rpc::request(
                9,
                methods::OPEN_MERGE,
                serde_json::json!({"tab": "m", "path": file, "text": text}),
            ),
        );
        let path = app.current_file.clone().expect("a merge tab in front");
        let tab = app.merge_tabs.get_mut(&path).expect("a merge tab");
        assert_eq!(tab.merge.unresolved(), 1);
        assert_eq!(tab.owner, Some(("test.tabs".to_string(), "m".to_string())));
        // The same tab id: no second tab.
        from_plugin(
            &mut app,
            Rpc::request(
                10,
                methods::OPEN_MERGE,
                serde_json::json!({"tab": "m", "path": file, "text": text}),
            ),
        );
        assert_eq!(app.merge_tabs.len(), 1);
        // Resolved: written to the disk.
        let tab = app.merge_tabs.get_mut(&path).unwrap();
        for chunk in tab.merge.conflicts_mut() {
            chunk.set(
                crate::diff::conflicts::Side::Yours,
                crate::diff::conflicts::Pick::Taken,
            );
            chunk.set(
                crate::diff::conflicts::Side::Theirs,
                crate::diff::conflicts::Pick::Ignored,
            );
        }
        assert_eq!(app.merge_tabs[&path].merge.unresolved(), 0);
        let rendered = app.merge_tabs[&path].merge.render();
        let tokio = tokio::runtime::Runtime::new().unwrap();
        tokio
            .block_on(merge_written(
                "test.tabs".into(),
                "m".into(),
                file.clone(),
                rendered,
            ))
            .unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "top\nmine\nend\n");
    }

    #[test]
    fn a_plugin_shows_a_text_in_a_tab_of_its_own_and_hears_its_diff_tab_closed() {
        let mut app = app_with_plugin();
        from_plugin(
            &mut app,
            Rpc::request(
                11,
                methods::OPEN_TEXT,
                serde_json::json!({"title": "a.rs @ abc123", "text": "fn old() {}\n", "extension": "rs"}),
            ),
        );
        let path = app.current_file.clone().expect("a tab in front");
        assert!(path.to_string_lossy().starts_with("rev://"));
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "fn old() {}\n"
        );
        // The lines of the cursor go with the file in front.
        let (first, last) = app.lines_to_edit(app.selection_span());
        assert_eq!((first, last), (0, 0));
        // A diff tab of the plugin, closed: it is forgotten.
        from_plugin(
            &mut app,
            Rpc::request(
                12,
                methods::OPEN_DIFF_TAB,
                serde_json::json!({"tab": "d", "title": "D", "files": [{"path": "a.rs", "patch": ""}]}),
            ),
        );
        let diff = app.current_file.clone().unwrap();
        assert!(app.diff_tabs.contains_key(&diff));
        let _ = app.close_path(diff.clone());
        assert!(!app.diff_tabs.contains_key(&diff));
    }

    #[test]
    fn the_tabs_of_a_plugin_are_grouped_under_it_and_their_changes_ask_before_quitting() {
        let mut app = app_with_plugin();
        from_plugin(
            &mut app,
            Rpc::request(
                21,
                methods::OPEN_TAB,
                serde_json::json!({"tab": "t1", "title": "orders"}),
            ),
        );
        let path = crate::plugins::tab_path("test.tabs", "t1", "orders");
        let group = app.tab_key(&path).expect("a group");
        assert_eq!(group, PathBuf::from("plugin-group://test.tabs"));
        assert!(app
            .tab_groups()
            .iter()
            .any(|g| g.project.as_ref() == Some(&group)));
        from_plugin(
            &mut app,
            Rpc::notification(
                methods::TAB_DIRTY,
                serde_json::json!({"tab": "t1", "dirty": true}),
            ),
        );
        app.overlay = ActiveOverlay::None;
        let _ = app.update(Message::Quit);
        assert!(matches!(app.overlay, ActiveOverlay::ConfirmQuit(_)));
    }

    #[test]
    fn the_tools_used_before_the_plugins_install_once_the_registry_is_there() {
        let config = Config {
            plugins_carried: false,
            lint_enabled: false,
            spell_check: false,
            local_history_days: 0,
            ..Config::default()
        };
        let mut app = ArdoiseApp::blank(config);
        let _ = app.plugins_carry();
        assert!(app.config.plugins_carried);
        let mut wanted = app.config.plugins_to_carry.clone();
        wanted.sort();
        let mut expected = vec![
            crate::plugins::migrate::GIT.to_string(),
            crate::plugins::migrate::TERMINAL.to_string(),
            crate::plugins::migrate::TASKS.to_string(),
        ];
        expected.sort();
        assert_eq!(wanted, expected);
        assert!(matches!(app.plugins.registry, Registry::Loading));
        // The registry: each one starts to install.
        let listed = |id: &str| crate::plugins::remote::Listed {
            id: id.into(),
            name: Default::default(),
            description: Default::default(),
            repository: format!("ardoise-editor/{id}"),
        };
        let _ = app.update(Message::Plugin(PluginMsg::RegistryLoaded(Ok(expected
            .iter()
            .map(|id| listed(id))
            .collect()))));
        for id in &expected {
            assert!(app.plugins.busy.contains(id), "{id}");
        }
        // Worked out once: a second start installs what is left only.
        let _ = app.plugins_carry();
        assert_eq!(app.config.plugins_to_carry.len(), 3);
    }
}
