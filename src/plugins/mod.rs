//! Plugins: programs of their own that add commands, panels and items of
//! the status bar to the editor. They are installed from the releases of a
//! GitHub repository ([`remote`]), kept on disk ([`store`]), run as
//! processes that speak JSON-RPC ([`host`]); what their panels show is
//! drawn by [`render`].

pub mod host;
pub mod remote;
pub mod render;
pub mod store;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use simpleedit_plugin::manifest::{Location, Manifest};
use simpleedit_plugin::protocol::{Settings, StatusParams};
use simpleedit_plugin::ui::Node;

pub use host::Incoming;
pub use remote::Listed;
pub use store::Installed;

/// A panel of a plugin: the plugin's id, the panel's id.
pub type PanelKey = (String, String);

/// What the plugins and the Plugins page send.
#[derive(Debug, Clone)]
pub enum PluginMsg {
    /// A plugin said something, or stopped.
    Incoming(Incoming),
    /// The button of a panel in the activity bar.
    TogglePanel(String, String),
    /// Closes the panel under the editor.
    CloseBottom,
    /// Something done in a panel: the plugin, the panel, the action and
    /// what the control holds.
    Ui(String, String, String, Option<Value>),
    /// A field of a panel typed in: the plugin, the panel, the field, the text.
    Input(String, String, String, String),
    /// A command of a plugin, run from the search for actions.
    Run(String, String),
    /// The answer to a plugin's question: the plugin, the request, yes / no.
    Answer(String, i64, bool),
    /// Starts a plugin that stopped, and opens again its panels.
    Restart(String),

    // ── The Plugins page ───────────────────────────────────────────────
    LoadRegistry,
    RegistryLoaded(Result<Vec<Listed>, String>),
    /// Installs (or updates) an official plugin of the list, by id.
    InstallOfficial(String),
    /// Updates a plugin that is not official, by id.
    Update(String),
    /// The end of an install: what it was for (an id or a repository).
    Installed(String, Result<Installed, String>),
    Uninstall(String),
    CheckUpdates,
    /// The newest version of each plugin that has a repository.
    UpdatesChecked(Vec<(String, Result<String, String>)>),
    /// The repository of a plugin of someone else, being typed.
    RepoTyped(String),
    /// Downloads that plugin to show what it is before it is installed.
    FetchRepo,
    Fetched(Result<Review, String>),
    /// Installs the plugin shown.
    AcceptReview,
    CancelReview,
    /// Links a folder (a plugin being written) as a plugin.
    LinkFolder,
    LinkPicked(Option<PathBuf>),
    OpenGuide,
}

/// A plugin of someone else, downloaded and checked, shown to the user
/// before it is installed.
#[derive(Debug, Clone)]
pub struct Review {
    pub manifest: Manifest,
    pub repository: String,
    pub bytes: Arc<Vec<u8>>,
}

/// Where the list of the official plugins is.
#[derive(Debug, Clone, Default)]
pub enum Registry {
    #[default]
    Unknown,
    Loading,
    Loaded(Vec<Listed>),
    Failed(String),
}

#[derive(Default)]
pub struct Plugins {
    pub installed: Vec<Installed>,
    /// Folders that hold no valid plugin, and why.
    pub broken: Vec<(String, String)>,
    pub registry: Registry,
    /// What is being installed, updated or removed (ids, or repositories).
    pub busy: HashSet<String>,
    /// What went wrong, by id or repository.
    pub errors: HashMap<String, String>,
    /// The newer version of a plugin, by id.
    pub updates: HashMap<String, String>,
    pub checking_updates: bool,
    pub repo_input: String,
    pub review: Option<Review>,
    /// What each panel shows, as the plugin last sent it.
    pub views: HashMap<PanelKey, Node>,
    /// The text of the fields of the panels as typed, which the plugin need
    /// not send back at each key.
    pub inputs: HashMap<(PanelKey, String), String>,
    /// The values the fields had in the last render of each panel.
    rendered_inputs: HashMap<PanelKey, HashMap<String, String>>,
    /// The panel of a plugin shown in the sidebar, and the one under the editor.
    pub sidebar: Option<PanelKey>,
    pub bottom: Option<PanelKey>,
    /// The items of the status bar: (plugin, item).
    pub status: BTreeMap<(String, String), StatusParams>,
    /// The plugins that stopped by themselves, and why.
    pub crashed: HashMap<String, String>,
    /// The files the plugins were told are open.
    pub told_open: HashSet<PathBuf>,
    /// The settings the plugins were last told.
    pub sent_settings: Option<Settings>,
}

impl Plugins {
    /// Reads what is installed.
    pub fn load() -> Self {
        let mut plugins = Plugins::default();
        plugins.reload();
        plugins
    }

    pub fn reload(&mut self) {
        let (installed, broken) = store::installed();
        self.installed = installed;
        self.broken = broken;
    }

    pub fn get(&self, id: &str) -> Option<&Installed> {
        self.installed.iter().find(|p| p.id() == id)
    }

    /// The panels of every plugin at `location`: (plugin, panel).
    pub fn panels(
        &self,
        location: Location,
    ) -> Vec<(&Installed, &simpleedit_plugin::manifest::Panel)> {
        self.installed
            .iter()
            .flat_map(|p| {
                p.manifest
                    .contributes
                    .panels
                    .iter()
                    .filter(move |panel| panel.location == location)
                    .map(move |panel| (p, panel))
            })
            .collect()
    }

    /// The title of a panel, in the language of the interface.
    pub fn panel_title(&self, key: &PanelKey) -> String {
        self.get(&key.0)
            .and_then(|p| p.manifest.contributes.panels.iter().find(|x| x.id == key.1))
            .map(|panel| panel.title.get(&rust_i18n::locale()).to_string())
            .unwrap_or_default()
    }

    /// Keeps what a panel shows; the fields the plugin changed take its
    /// text, the others keep what was typed.
    pub fn set_view(&mut self, key: PanelKey, root: Node) {
        let mut now = HashMap::new();
        collect_inputs(&root, &mut now);
        let before = self.rendered_inputs.remove(&key).unwrap_or_default();
        for (id, value) in &now {
            if before.get(id) != Some(value) {
                self.inputs.insert((key.clone(), id.clone()), value.clone());
            }
        }
        self.rendered_inputs.insert(key.clone(), now);
        self.views.insert(key, root);
    }

    /// Forgets what a plugin showed (it stopped or was removed).
    pub fn forget(&mut self, id: &str) {
        self.views.retain(|k, _| k.0 != id);
        self.inputs.retain(|k, _| k.0 .0 != id);
        self.rendered_inputs.retain(|k, _| k.0 != id);
        self.status.retain(|k, _| k.0 != id);
    }

    /// The text of a field as shown.
    pub fn input_text<'a>(&'a self, key: &PanelKey, id: &str, rendered: &'a str) -> &'a str {
        self.inputs
            .get(&(key.clone(), id.to_string()))
            .map_or(rendered, String::as_str)
    }

    /// Whether a panel of this plugin is open.
    pub fn has_open_panel(&self, id: &str) -> bool {
        self.sidebar.as_ref().is_some_and(|k| k.0 == id)
            || self.bottom.as_ref().is_some_and(|k| k.0 == id)
    }
}

fn collect_inputs(node: &Node, out: &mut HashMap<String, String>) {
    match node {
        Node::Input { id, value, .. } => {
            out.insert(id.clone(), value.clone());
        }
        Node::Column { children, .. } | Node::Row { children, .. } => {
            for child in children {
                collect_inputs(child, out);
            }
        }
        Node::Scroll { child } => collect_inputs(child, out),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simpleedit_plugin::ui::{column, input};

    #[test]
    fn typed_text_stays_until_the_plugin_changes_the_field() {
        let mut plugins = Plugins::default();
        let key: PanelKey = ("acme.a".into(), "p".into());
        plugins.set_view(key.clone(), column([input("name", "", "")]));
        assert_eq!(plugins.input_text(&key, "name", ""), "");
        plugins
            .inputs
            .insert((key.clone(), "name".into()), "Ada".into());
        // A render that does not touch the field keeps what was typed.
        plugins.set_view(key.clone(), column([input("name", "", "")]));
        assert_eq!(plugins.input_text(&key, "name", ""), "Ada");
        // A render that sets it wins.
        plugins.set_view(key.clone(), column([input("name", "Bob", "")]));
        assert_eq!(plugins.input_text(&key, "name", "Bob"), "Bob");
        plugins.forget("acme.a");
        assert!(plugins.views.is_empty() && plugins.inputs.is_empty());
    }
}
