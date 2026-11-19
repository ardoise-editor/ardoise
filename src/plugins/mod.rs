//! Plugins: programs of their own that add commands, panels and items of
//! the status bar to the editor. They are installed from the releases of a
//! GitHub repository ([`remote`]), kept on disk ([`store`]), run as
//! processes that speak JSON-RPC ([`host`]); what their panels show is
//! drawn by [`render`].

pub mod grid;
pub mod host;
pub mod migrate;
pub mod remote;
pub mod render;
pub mod screen;
pub mod store;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
pub use simpleedit_plugin::manifest::Menu;
use simpleedit_plugin::manifest::{Location, Manifest};
use simpleedit_plugin::protocol::{Settings, StatusParams};
use simpleedit_plugin::ui::Node;

pub use host::Incoming;
pub use remote::Listed;
pub use store::Installed;

/// A panel of a plugin: the plugin's id, the panel's id.
pub type PanelKey = (String, String);

/// The completions a plugin gave for the cursor of one of its editors.
#[derive(Debug, Clone)]
pub struct NodeCompletion {
    pub editor: grid::GridKey,
    pub items: Vec<simpleedit_plugin::protocol::CompletionItem>,
    /// Where the replaced word starts, in characters.
    pub from: usize,
    /// The cursor when it was asked, in characters.
    pub at: usize,
    pub picked: usize,
}

/// What a file is compared with to mark its changed lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base {
    /// This text (none: the file is new, every line added).
    Text(Option<String>),
    /// No marks at all.
    None,
}

/// A tab of a plugin among the files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginTab {
    pub plugin: String,
    pub tab: String,
    /// It holds changes not kept.
    pub dirty: bool,
    /// The plugin was asked what it shows (or showed it already).
    pub asked: bool,
}

/// The path a tab of a plugin goes by: `plugin://<plugin>/<tab>/<title>`.
pub fn tab_path(plugin: &str, tab: &str, title: &str) -> PathBuf {
    let title = title.replace('/', "∕");
    PathBuf::from(format!("plugin://{plugin}/{tab}/{title}"))
}

/// The title of a tab of a plugin, from its path.
pub fn tab_title(path: &std::path::Path) -> Option<String> {
    let rest = path.to_str()?.strip_prefix("plugin://")?;
    let mut parts = rest.splitn(3, '/');
    let (_, _, title) = (parts.next()?, parts.next()?, parts.next()?);
    Some(title.replace('∕', "/"))
}

/// The plugin and the tab of a path of a tab of a plugin.
pub fn tab_of(path: &std::path::Path) -> Option<(String, String)> {
    let rest = path.to_str()?.strip_prefix("plugin://")?;
    let mut parts = rest.splitn(3, '/');
    Some((parts.next()?.to_string(), parts.next()?.to_string()))
}

/// A command of a plugin in a context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub menu: simpleedit_plugin::manifest::Menu,
    pub plugin: String,
    pub command: String,
    pub label: String,
}

impl MenuItem {
    /// What its button sends, for the file or folder the menu is on.
    pub fn message(&self, path: &std::path::Path) -> crate::app::Message {
        crate::app::Message::Plugin(PluginMsg::RunAt(
            self.plugin.clone(),
            self.command.clone(),
            path.to_path_buf(),
        ))
    }
}

/// What the plugins and the Plugins page send.
#[derive(Debug, Clone)]
pub enum PluginMsg {
    /// A plugin said something, or stopped.
    Incoming(Incoming),
    /// The boxes the plugins add to the question before a file is deleted:
    /// (plugin, box).
    DeleteOptions(
        PathBuf,
        Vec<(String, simpleedit_plugin::protocol::DeleteOption)>,
    ),
    /// A box of that question ticked or not.
    DeleteOptionToggled(usize, bool),
    /// The button of a panel in the activity bar.
    TogglePanel(String, String),
    /// Closes the panel under the editor.
    CloseBottom,
    /// Something done in a panel: the plugin, the panel, the action and
    /// what the control holds.
    Ui(String, String, String, Option<Value>),
    /// A field of a panel typed in: the plugin, the panel, the field, the text.
    Input(String, String, String, String),
    /// Something done in an editor of a panel: the plugin, the panel, the
    /// editor, the action.
    EditorAction(String, String, String, iced::widget::text_editor::Action),
    /// A command of a plugin, run from the search for actions.
    Run(String, String),
    /// A command of a plugin, run from the context menu of a file or folder.
    RunAt(String, String, PathBuf),
    /// The answer to a plugin's question: the plugin, the request, yes / no.
    Answer(String, i64, bool),
    /// The answer to a request of a plugin, worked out in the background.
    Respond(
        String,
        i64,
        Result<Value, simpleedit_plugin::protocol::RpcError>,
    ),
    /// Starts a plugin that stopped, and opens again its panels.
    Restart(String),
    /// Shows or hides the settings of a plugin in Preferences › Plugins.
    ToggleSettings(String),
    /// A screen of a panel takes the keyboard, or gives it back: the
    /// plugin, the panel, the screen.
    ScreenFocus(String, String, String, bool),
    /// The text selected on a screen, for the clipboard.
    ScreenCopy(String),
    /// A paste on a screen (wrapped as a bracketed paste or not).
    ScreenPaste(String, String, String, bool),
    ScreenPasted(String, String, String, bool, Option<String>),
    /// A click on `file:line` on a screen, and the folder it is from.
    ScreenOpen(String, PathBuf),
    /// A grid of a panel: the plugin, the panel, the grid.
    Grid(String, String, String, grid::GridMsg),
    /// The pointer dragging the edge of a column (x), and let go.
    GridResize(f32),
    GridResizeEnd,
    /// The text of the clipboard for a paste in the grid that has the keyboard.
    GridPasted(Option<String>),
    /// A completion clicked under an editor of a panel.
    CompletionPick(usize),
    /// What may come at the cursor of an editor of a panel, as the plugin
    /// answered.
    Completed(
        grid::GridKey,
        usize,
        Result<simpleedit_plugin::protocol::Completions, String>,
    ),

    // ── The Plugins page ───────────────────────────────────────────────
    LoadRegistry,
    RegistryLoaded(Result<Vec<Listed>, String>),
    /// Installs (or updates) an official plugin of the list, by id.
    InstallOfficial(String),
    /// Updates a plugin that is not official, by id.
    Update(String),
    /// The end of an install: what it was for (an id or a repository).
    Installed(String, Box<Result<Installed, String>>),
    Uninstall(String),
    /// The question before a plugin is removed: whether its data goes too.
    AskUninstall(String),
    UninstallDataToggled(bool),
    /// Removes the plugin and its data: it is asked first to take its
    /// passwords out of the keychain.
    UninstallWithData(String),
    /// Its answer (an error: the keychain may still hold some).
    DataForgotten(String, Result<(), String>),
    CheckUpdates,
    /// The newest version of each plugin that has a repository.
    UpdatesChecked(Vec<(String, Result<String, String>)>),
    /// The repository of a plugin of someone else, being typed.
    RepoTyped(String),
    /// Downloads that plugin to show what it is before it is installed.
    FetchRepo,
    Fetched(Box<Result<Review, String>>),
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
    /// The check of the plugins was asked with the app's: the newer ones
    /// install as soon as they are known.
    pub update_all: bool,
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
    /// The settings panel open in Preferences › Plugins.
    pub settings_open: Option<PanelKey>,
    /// The screen that has the keyboard: its panel and its id.
    pub screen_focus: Option<(PanelKey, String)>,
    /// What is only seen of the grids of the panels, and the one that has
    /// the keyboard.
    pub grids: HashMap<grid::GridKey, grid::GridView>,
    pub grid_focus: Option<grid::GridKey>,
    /// The editor of a panel last typed or clicked in, and the list of
    /// completions open under it.
    pub editor_focus: Option<grid::GridKey>,
    pub completion: Option<NodeCompletion>,
    /// How files stand, as each plugin said it.
    pub decorations: BTreeMap<String, Vec<simpleedit_plugin::protocol::FileDecoration>>,
    /// The texts files are compared with to mark their changed lines: the
    /// text (none: new), or no marks at all.
    pub bases: HashMap<PathBuf, Base>,
    /// Where files come from, shown in place of their folder (a copy of a
    /// remote file: `host:/path`).
    pub origins: HashMap<PathBuf, String>,
    /// The panel the user clicked in last (its buttons' keys work), until
    /// the keyboard goes elsewhere.
    pub panel_focus: Option<PanelKey>,
    /// The selection each editor of a panel was last given by its plugin.
    editor_selections: HashMap<(PanelKey, String), simpleedit_plugin::ui::EditorSelection>,
    /// What each plugin offers in Search Everywhere.
    pub search_items: BTreeMap<String, Vec<simpleedit_plugin::protocol::SearchItem>>,
    /// A line pointed at in the gutter (right click): the next answer to
    /// `editor/activeFile` gives it as the lines of the cursor.
    pub pointed_line: Option<usize>,
    /// The plugin whose annotations are the column of the file in front.
    pub annotations_of: Option<String>,
    /// The tabs of the plugins among the files, by path.
    pub tabs: BTreeMap<PathBuf, PluginTab>,
    /// The text of the editors of the panels, and the value each had in the
    /// last render.
    pub editors: HashMap<(PanelKey, String), (iced::widget::text_editor::Content, String)>,
    /// The items of the status bar: (plugin, item).
    pub status: BTreeMap<(String, String), StatusParams>,
    /// The plugins that stopped by themselves, and why.
    pub crashed: HashMap<String, String>,
    /// The files the plugins were told are open.
    pub told_open: HashSet<PathBuf>,
    /// The settings the plugins were last told.
    pub sent_settings: Option<Settings>,
    /// The file in front the plugins were last told of.
    pub sent_active: Option<Option<PathBuf>>,
    /// The problems the plugins published, by file and plugin.
    pub published: BTreeMap<PathBuf, BTreeMap<String, Vec<crate::lint::Diagnostic>>>,
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
        // The editors: their text follows the plugin's when it changes it.
        let mut editors = Vec::new();
        collect_editors(&root, &mut editors);
        for (id, value, selection) in editors {
            let slot = (key.clone(), id);
            let moved = selection.filter(|s| self.editor_selections.get(&slot) != Some(s));
            match self.editors.get_mut(&slot) {
                Some((content, last)) if *last == value => {
                    let _ = content;
                }
                Some((content, last)) => {
                    *content = iced::widget::text_editor::Content::with_text(&value);
                    *last = value;
                }
                None => {
                    let content = iced::widget::text_editor::Content::with_text(&value);
                    self.editors.insert(slot.clone(), (content, value));
                }
            }
            match (moved, self.editors.get_mut(&slot)) {
                (Some(selection), Some((content, _))) => {
                    content.move_to(editor_cursor(&content.text(), selection));
                    self.editor_selections.insert(slot, selection);
                }
                _ if selection.is_none() => {
                    self.editor_selections.remove(&slot);
                }
                _ => {}
            }
        }
        self.views.insert(key, root);
    }

    /// Forgets what a plugin showed (it stopped or was removed).
    pub fn forget(&mut self, id: &str) {
        self.views.retain(|k, _| k.0 != id);
        self.inputs.retain(|k, _| k.0 .0 != id);
        self.editors.retain(|k, _| k.0 .0 != id);
        self.editor_selections.retain(|k, _| k.0 .0 != id);
        if self.panel_focus.as_ref().is_some_and(|k| k.0 == id) {
            self.panel_focus = None;
        }
        self.rendered_inputs.retain(|k, _| k.0 != id);
        self.status.retain(|k, _| k.0 != id);
        for by_plugin in self.published.values_mut() {
            by_plugin.remove(id);
        }
        self.published.retain(|_, by_plugin| !by_plugin.is_empty());
    }

    /// The problems the plugins published for a file.
    pub fn published_for(&self, path: &std::path::Path) -> Vec<crate::lint::Diagnostic> {
        self.published
            .get(path)
            .into_iter()
            .flat_map(|by_plugin| by_plugin.values().flatten().cloned())
            .collect()
    }

    /// The text of a field as shown.
    pub fn input_text<'a>(&'a self, key: &PanelKey, id: &str, rendered: &'a str) -> &'a str {
        self.inputs
            .get(&(key.clone(), id.to_string()))
            .map_or(rendered, String::as_str)
    }

    /// Whether a panel of this plugin is open.
    pub fn has_open_panel(&self, id: &str) -> bool {
        [&self.sidebar, &self.bottom, &self.settings_open]
            .into_iter()
            .any(|k| k.as_ref().is_some_and(|k| k.0 == id))
    }

    /// The plugin installed that finds problems in files (Lint): it has a
    /// page of its own in the preferences.
    pub fn linter(&self) -> Option<&Installed> {
        self.installed
            .iter()
            .find(|p| p.manifest.contributes.linter.is_some())
    }

    /// The panel of a plugin's settings, if it has one.
    pub fn settings_panel(&self, id: &str) -> Option<&simpleedit_plugin::manifest::Panel> {
        self.get(id)?
            .manifest
            .contributes
            .panels
            .iter()
            .find(|p| p.location == Location::Settings)
    }
}

/// The cursor of an editor for a selection given in characters (the
/// editor counts bytes).
fn editor_cursor(
    text: &str,
    selection: simpleedit_plugin::ui::EditorSelection,
) -> iced::widget::text_editor::Cursor {
    use iced::widget::text_editor::Position;
    let lines: Vec<&str> = text.split('\n').collect();
    let at = |p: simpleedit_plugin::ui::EditorPos| {
        let line = p.line.min(lines.len().saturating_sub(1));
        let s = lines.get(line).copied().unwrap_or_default();
        let column = s.char_indices().nth(p.column).map_or(s.len(), |(i, _)| i);
        Position { line, column }
    };
    let (anchor, cursor) = (at(selection.anchor), at(selection.cursor));
    iced::widget::text_editor::Cursor {
        position: cursor,
        selection: (anchor != cursor).then_some(anchor),
    }
}

/// The button of a panel that a key presses, as its `key` names it
/// (`"Ctrl+Shift+C"`, `"F4"`): its action.
pub fn keyed_button(node: &Node, key: &str, ctrl: bool, alt: bool, shift: bool) -> Option<String> {
    match node {
        Node::Button {
            action,
            enabled: true,
            key: Some(spec),
            ..
        } if key_matches(spec, key, ctrl, alt, shift) => Some(action.clone()),
        Node::Column { children, .. } | Node::Row { children, .. } => children
            .iter()
            .find_map(|c| keyed_button(c, key, ctrl, alt, shift)),
        Node::Scroll { child, .. } => keyed_button(child, key, ctrl, alt, shift),
        _ => None,
    }
}

/// Whether a key pressed is the one a plugin named: its modifiers, then the
/// key (letters in either case). `Ctrl` is Cmd on macOS.
pub fn key_matches(spec: &str, key: &str, ctrl: bool, alt: bool, shift: bool) -> bool {
    let mut parts: Vec<&str> = spec.split('+').map(str::trim).collect();
    let Some(name) = parts.pop() else {
        return false;
    };
    let has = |m: &str| parts.iter().any(|p| p.eq_ignore_ascii_case(m));
    let wants_ctrl = has("ctrl") || has("cmd");
    !name.is_empty()
        && name.eq_ignore_ascii_case(key)
        && wants_ctrl == ctrl
        && has("alt") == alt
        && has("shift") == shift
}

fn collect_editors(
    node: &Node,
    out: &mut Vec<(
        String,
        String,
        Option<simpleedit_plugin::ui::EditorSelection>,
    )>,
) {
    match node {
        Node::Editor {
            id,
            value,
            selection,
            ..
        } => out.push((id.clone(), value.clone(), *selection)),
        Node::Column { children, .. } | Node::Row { children, .. } => {
            for child in children {
                collect_editors(child, out);
            }
        }
        Node::Scroll { child, .. } => collect_editors(child, out),
        _ => {}
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
        Node::Scroll { child, .. } => collect_inputs(child, out),
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

    #[test]
    fn a_selection_sent_by_the_plugin_moves_the_cursor_once() {
        use simpleedit_plugin::ui::{editor, EditorPos, EditorSelection};
        let mut plugins = Plugins::default();
        let key: PanelKey = ("acme.a".into(), "p".into());
        let found = EditorSelection {
            anchor: EditorPos { line: 1, column: 1 },
            cursor: EditorPos { line: 1, column: 3 },
        };
        let text = "abc\néty";
        plugins.set_view(
            key.clone(),
            column([editor("e", text, "").selection(found)]),
        );
        let slot = (key.clone(), "e".to_string());
        let cursor = plugins.editors[&slot].0.cursor();
        // In bytes for the editor: "é" is two.
        assert_eq!((cursor.position.line, cursor.position.column), (1, 4));
        assert_eq!(cursor.selection.map(|p| p.column), Some(2));
        // The user moves; the same selection sent again leaves the cursor.
        plugins
            .editors
            .get_mut(&slot)
            .unwrap()
            .0
            .move_to(iced::widget::text_editor::Cursor {
                position: iced::widget::text_editor::Position { line: 0, column: 0 },
                selection: None,
            });
        plugins.set_view(
            key.clone(),
            column([editor("e", text, "").selection(found)]),
        );
        assert_eq!(plugins.editors[&slot].0.cursor().position.line, 0);
    }

    #[test]
    fn a_key_presses_the_button_that_names_it() {
        use simpleedit_plugin::ui::button;
        let root = column([
            button("Copy", "copy").key("Ctrl+C"),
            button("Off", "off").key("F4").enabled(false),
            button("Form", "form").key("shift+f4"),
        ]);
        assert_eq!(
            keyed_button(&root, "c", true, false, false).as_deref(),
            Some("copy")
        );
        assert_eq!(
            keyed_button(&root, "C", true, false, false).as_deref(),
            Some("copy")
        );
        assert_eq!(keyed_button(&root, "c", false, false, false), None);
        assert_eq!(keyed_button(&root, "c", true, false, true), None);
        assert_eq!(
            keyed_button(&root, "F4", false, false, false),
            None,
            "disabled"
        );
        assert_eq!(
            keyed_button(&root, "F4", false, false, true).as_deref(),
            Some("form")
        );
        assert!(key_matches("Cmd+S", "s", true, false, false));
        assert!(!key_matches("", "", false, false, false));
    }
}
