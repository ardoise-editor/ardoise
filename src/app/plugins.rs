//! The plugins in the app: starting them, what they ask and say, their
//! panels, commands and items of the status bar, the files and settings
//! they follow, and the installs of the Plugins page.

use std::sync::Arc;

use super::*;
use crate::plugins::{
    host, remote, store, Incoming, Installed, PanelKey, PluginMsg, Registry, Review,
};
use serde_json::Value;
use simpleedit_plugin::manifest::{Event, Location};
use simpleedit_plugin::protocol::{
    self as rpc, codes, methods, ActiveFile, CommandParams, ConfirmParams, Context, FileParams,
    InitializeParams, Level, Message as Rpc, NotifyParams, OpenFileParams, PanelParams,
    RenderParams, RpcError, Settings, StatusParams, UiEvent,
};

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

impl SimpleEditApp {
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
    fn plugin_ensure(&mut self, id: &str) -> bool {
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
                        host::notify(id, methods::FILE_OPENED, FileParams { path: path.clone() });
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

    /// A file was opened, saved or closed: the plugins that asked hear of it.
    pub(super) fn plugins_file_event(&mut self, event: Event, path: &Path) {
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
            Event::Startup => return,
        };
        for plugin in &self.plugins.installed {
            if plugin.manifest.events.contains(&event) {
                host::notify(
                    plugin.id(),
                    method,
                    FileParams {
                        path: path.to_path_buf(),
                    },
                );
            }
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
                    database: false,
                    api: false,
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
                self.plugins.bottom = Some(key);
            }
            None => {}
        }
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
                if let Some(old) = self.plugins.bottom.take() {
                    self.close_plugin_panel(&old);
                }
                Task::none()
            }
            PluginMsg::Ui(plugin, panel, action, value) => {
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
            PluginMsg::Input(plugin, panel, field, value) => {
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
            PluginMsg::Answer(plugin, request, yes) => {
                self.overlay = ActiveOverlay::None;
                host::respond(&plugin, request, Ok(Value::Bool(yes)));
                Task::none()
            }
            PluginMsg::Restart(id) => {
                if self.plugin_ensure(&id) {
                    let open: Vec<PanelKey> =
                        [self.plugins.sidebar.clone(), self.plugins.bottom.clone()]
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
                Task::none()
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
                    plugin_msg(PluginMsg::Installed(key.clone(), r))
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
                    plugin_msg(PluginMsg::Installed(key.clone(), r))
                })
            }
            PluginMsg::Installed(key, result) => {
                self.plugins.busy.remove(&key);
                match result {
                    Ok(installed) => {
                        let id = installed.id().to_string();
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
                        for slot in [&mut self.plugins.sidebar, &mut self.plugins.bottom] {
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
            PluginMsg::Uninstall(id) => {
                host::stop(&id);
                match store::uninstall(&id) {
                    Ok(()) => {
                        self.plugins.forget(&id);
                        self.plugins.crashed.remove(&id);
                        self.plugins.updates.remove(&id);
                        self.plugins.errors.remove(&id);
                        for slot in [&mut self.plugins.sidebar, &mut self.plugins.bottom] {
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
                if list.is_empty() || self.plugins.checking_updates {
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
                Task::perform(fetch_review(repo), |r| plugin_msg(PluginMsg::Fetched(r)))
            }
            PluginMsg::Fetched(result) => {
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
                    |r| plugin_msg(PluginMsg::Installed("repo".into(), r)),
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
                self.plugin_update(PluginMsg::Installed("link".into(), linked))
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
                            });
                            if known {
                                self.plugins.set_view((id, p.panel), p.root);
                            }
                        }
                    }
                    methods::NOTIFY => {
                        if let Ok(p) = serde_json::from_value::<NotifyParams>(params) {
                            self.set_status(p.message, p.level == Level::Error);
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
                Task::none()
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
                methods::ACTIVE_FILE => {
                    let active = ActiveFile {
                        path: self.plugin_context().file,
                        text: crate::editor::buffer_text(&self.editor.content),
                        selection: self.editor.content.selection(),
                        language: self.editor.language.clone(),
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
    pub(super) fn plugin_tools(&self, location: Location) -> Vec<(String, bool, String, Message)> {
        let open = match location {
            Location::Sidebar => self.plugins.sidebar.as_ref().filter(|_| self.show_sidebar),
            Location::Bottom => self.plugins.bottom.as_ref(),
        };
        let locale = rust_i18n::locale();
        self.plugins
            .panels(location)
            .into_iter()
            .map(|(plugin, panel)| {
                let title = panel.title.get(&locale).to_string();
                let icon = panel.icon.clone().unwrap_or_else(|| {
                    title
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_default()
                });
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
        container(
            iced::widget::column![
                text(name).size(15).color(p.text),
                text(q.message.clone())
                    .size(12.5)
                    .color(p.muted)
                    .shaping(iced::widget::text::Shaping::Advanced),
                row![
                    button(text(t!("confirm.cancel").to_string()).size(13))
                        .padding([7, 16])
                        .on_press(no)
                        .style((se_theme::GhostButton { dark, active: true }).style()),
                    Space::new().width(Length::Fill),
                    button(
                        text(
                            q.confirm
                                .clone()
                                .unwrap_or_else(|| t!("plugins.ok").to_string())
                        )
                        .size(13)
                    )
                    .padding([7, 16])
                    .on_press(yes)
                    .style((se_theme::Primary { dark }).style()),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(14)
            .padding(24),
        )
        .width(380)
        .style(se_theme::card(dark))
        .into()
    }

    /// Escape or a click beside the question: it is answered no.
    pub(super) fn dismiss_plugin_question(&mut self) {
        if let ActiveOverlay::PluginConfirm(q) = &self.overlay {
            host::respond(&q.plugin, q.request, Ok(Value::Bool(false)));
        }
    }
}

fn invalid(e: impl std::fmt::Display) -> RpcError {
    RpcError {
        code: codes::INVALID_PARAMS,
        message: e.to_string(),
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

        let mut app = SimpleEditApp::blank(Config::default());
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
            Some(&simpleedit_plugin::ui::text("hi"))
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
        let mut app = SimpleEditApp::blank(Config::default());
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
