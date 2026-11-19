//! The grids of the panels of plugins, and the completions of their
//! editors: what the editor does with them before telling the plugin.

use super::*;
use crate::plugins::grid::{self, GridKey, GridMsg};
use crate::plugins::{host, NodeCompletion, PluginMsg};
use ardoise_plugin::protocol::{methods, CompletionParams, Completions, UiEvent};
use ardoise_plugin::ui::{GridEvent, Node};

/// How long a plugin has to answer for completions.
const COMPLETION_WAIT: std::time::Duration = std::time::Duration::from_secs(3);

/// The cursor of a text, in characters from its start.
fn offset_of(text: &str, line: usize, column: usize) -> usize {
    let mut offset = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i == line {
            return offset + column.min(l.chars().count());
        }
        offset += l.chars().count() + 1;
    }
    offset
}

/// Where a character of a text is: line, column.
fn position_of(text: &str, offset: usize) -> (usize, usize) {
    let mut left = offset;
    for (i, l) in text.split('\n').enumerate() {
        let n = l.chars().count();
        if left <= n {
            return (i, left);
        }
        left -= n + 1;
    }
    let lines: Vec<&str> = text.split('\n').collect();
    (
        lines.len().saturating_sub(1),
        lines.last().map_or(0, |l| l.chars().count()),
    )
}

/// The start of the word (letters, digits, `_`) that ends at `offset`.
fn word_start(text: &str, offset: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut start = offset.min(chars.len());
    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }
    start
}

/// An editor of a panel in a tree of nodes, and whether it asks for
/// completions.
fn editor_completes(node: &Node, id: &str) -> bool {
    match node {
        Node::Editor {
            id: this,
            completion,
            ..
        } => this == id && *completion,
        Node::Column { children, .. } | Node::Row { children, .. } => {
            children.iter().any(|c| editor_completes(c, id))
        }
        Node::Scroll { child, .. } => editor_completes(child, id),
        _ => false,
    }
}

impl ArdoiseApp {
    fn grid_node(&self, key: &GridKey) -> Option<&ardoise_plugin::ui::Grid> {
        let root = self.plugins.views.get(&key.0)?;
        grid::find(root, &key.1)
    }

    fn grid_tell(&self, key: &GridKey, event: GridEvent) {
        host::notify(
            &key.0 .0,
            methods::UI_EVENT,
            UiEvent {
                panel: key.0 .1.clone(),
                action: key.1.clone(),
                value: serde_json::to_value(event).ok(),
            },
        );
    }

    pub(super) fn plugin_grid(
        &mut self,
        plugin: String,
        panel: String,
        id: String,
        m: GridMsg,
    ) -> Task<Message> {
        let key: GridKey = ((plugin, panel), id);
        let rows = self.grid_node(&key).map_or(0, |g| g.rows.len());
        match m {
            GridMsg::Scrolled(offset, height) => {
                let view = self.plugins.grids.entry(key.clone()).or_default();
                view.scroll = (offset, height);
                // The last rows are in view: the plugin may give more.
                let end = rows as f32 * grid::ROW_HEIGHT;
                if rows > 0
                    && offset + height >= end - 2.0 * grid::ROW_HEIGHT
                    && view.end_sent != rows
                {
                    view.end_sent = rows;
                    self.grid_tell(&key, GridEvent::End);
                }
                Task::none()
            }
            GridMsg::ScrolledAcross(offset, width) => {
                self.plugins.grids.entry(key).or_default().across = (offset, width);
                Task::none()
            }
            GridMsg::Press(row, col) => {
                self.plugins.grid_focus = Some(key.clone());
                self.plugins.editor_focus = None;
                let anchor = match (
                    self.mod_shift,
                    self.grid_node(&key).and_then(|g| g.selection),
                ) {
                    (true, Some(s)) => s.anchor,
                    _ => (row, col),
                };
                self.grid_tell(
                    &key,
                    GridEvent::Select {
                        anchor,
                        cursor: (row, col),
                    },
                );
                Task::none()
            }
            GridMsg::Open(row, col) => {
                self.plugins.grid_focus = Some(key.clone());
                self.grid_tell(&key, GridEvent::Open { row, col });
                iced::widget::operation::focus(grid::edit_id())
            }
            GridMsg::Sort(col) => {
                self.grid_tell(&key, GridEvent::Sort { col });
                Task::none()
            }
            GridMsg::ResizeStart(col) => {
                let width = self
                    .grid_node(&key)
                    .and_then(|g| g.columns.get(col).and_then(|c| c.width))
                    .unwrap_or(160.0);
                let view = self.plugins.grids.entry(key).or_default();
                let width = view.widths.get(&col).copied().unwrap_or(width);
                view.resizing = Some((col, width, None));
                Task::none()
            }
            GridMsg::EditInput(value) => {
                self.grid_tell(&key, GridEvent::Edit { value });
                Task::none()
            }
            GridMsg::EditCommit => {
                self.grid_tell(&key, GridEvent::Commit);
                Task::none()
            }
            GridMsg::Filter(col, value) => {
                self.grid_tell(
                    &key,
                    GridEvent::Filter {
                        col,
                        value,
                        submit: false,
                    },
                );
                Task::none()
            }
            GridMsg::FilterSubmit(col) => {
                let value = self
                    .grid_node(&key)
                    .and_then(|g| g.columns.get(col).map(|c| c.filter.clone()))
                    .unwrap_or_default();
                self.grid_tell(
                    &key,
                    GridEvent::Filter {
                        col,
                        value,
                        submit: true,
                    },
                );
                Task::none()
            }
            GridMsg::Menu(action, row, col) => {
                host::notify(
                    &key.0 .0,
                    methods::UI_EVENT,
                    UiEvent {
                        panel: key.0 .1.clone(),
                        action,
                        value: Some(serde_json::json!({"row": row, "col": col})),
                    },
                );
                Task::none()
            }
        }
    }

    /// A panel was drawn again: a grid whose cursor moved out of view
    /// scrolls to it.
    pub(super) fn plugin_grids_follow(&mut self, key: &crate::plugins::PanelKey) -> Task<Message> {
        let Some(root) = self.plugins.views.get(key) else {
            return Task::none();
        };
        let cursors: Vec<(String, Option<(usize, usize)>)> = grid::all(root)
            .into_iter()
            .map(|g| (g.id.clone(), g.selection.map(|s| s.cursor)))
            .collect();
        let mut tasks = Vec::new();
        for (id, cursor) in cursors {
            let view = self
                .plugins
                .grids
                .entry((key.clone(), id.clone()))
                .or_default();
            if view.cursor == cursor {
                continue;
            }
            view.cursor = cursor;
            let Some((row, _)) = cursor else {
                continue;
            };
            if let Some(y) = grid::follow(row, view.scroll.0, view.scroll.1) {
                view.scroll.0 = y;
                tasks.push(iced::widget::operation::scroll_to(
                    grid::scroll_id(key, &id),
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
                ));
            }
        }
        Task::batch(tasks)
    }

    /// The pointer drags the edge of a column.
    pub(super) fn plugin_grid_resize(&mut self, x: f32) {
        for view in self.plugins.grids.values_mut() {
            if let Some((col, width, start)) = view.resizing {
                match start {
                    None => view.resizing = Some((col, width, Some(x))),
                    Some(start) => {
                        view.widths.insert(col, grid::dragged(width, x - start));
                    }
                }
            }
        }
    }

    /// The edge of a column let go: the plugin hears its new width.
    pub(super) fn plugin_grid_resize_end(&mut self) {
        let done: Vec<(GridKey, usize, f32)> = self
            .plugins
            .grids
            .iter_mut()
            .filter_map(|(key, view)| {
                let (col, _, _) = view.resizing.take()?;
                Some((key.clone(), col, *view.widths.get(&col)?))
            })
            .collect();
        for (key, col, width) in done {
            self.grid_tell(&key, GridEvent::Resize { col, width });
        }
    }

    /// Whether a column of a grid is being dragged wider.
    pub(super) fn plugin_grid_resizing(&self) -> bool {
        self.plugins.grids.values().any(|v| v.resizing.is_some())
    }

    /// A key, for the grid that has the keyboard (none: not for it).
    pub(super) fn plugin_grid_key(
        &mut self,
        key: &str,
        ctrl: bool,
        alt: bool,
        shift: bool,
    ) -> Option<Task<Message>> {
        let focus = self.plugins.grid_focus.clone()?;
        let grid = self.grid_node(&focus)?;
        // An item of its menu named by the key, on the cell of the cursor.
        if let Some((action, row, col)) = grid::menu_key(grid, key, ctrl, alt, shift) {
            let ((plugin, panel), id) = focus;
            return Some(self.plugin_grid(plugin, panel, id, GridMsg::Menu(action, row, col)));
        }
        if ctrl && key.eq_ignore_ascii_case("v") && grid.editing.is_none() {
            return Some(
                iced::clipboard::read().map(|t| Message::Plugin(PluginMsg::GridPasted(t))),
            );
        }
        let event = grid::key(grid, key, ctrl, shift)?;
        self.grid_tell(&focus, event);
        Some(Task::none())
    }

    /// A key named by a button of the panel the user clicked in last.
    pub(super) fn plugin_panel_key(
        &mut self,
        key: &str,
        ctrl: bool,
        alt: bool,
        shift: bool,
    ) -> Option<Task<Message>> {
        if self.plugins.editor_focus.is_some() || self.plugins.grid_focus.is_some() {
            return None;
        }
        let focus = self.plugins.panel_focus.clone()?;
        let root = self.plugins.views.get(&focus)?;
        let action = crate::plugins::keyed_button(root, key, ctrl, alt, shift)?;
        host::notify(
            &focus.0,
            methods::UI_EVENT,
            UiEvent {
                panel: focus.1,
                action,
                value: None,
            },
        );
        Some(Task::none())
    }

    pub(super) fn plugin_grid_pasted(&mut self, text: Option<String>) {
        if let (Some(focus), Some(text)) = (self.plugins.grid_focus.clone(), text) {
            self.grid_tell(&focus, GridEvent::Paste { text });
        }
    }

    /// An editor of a panel was typed or clicked in: it has the keyboard, and
    /// a letter typed asks again what may come.
    pub(super) fn plugin_editor_touched(
        &mut self,
        slot: GridKey,
        typed: Option<char>,
    ) -> Task<Message> {
        self.plugins.grid_focus = None;
        self.plugins.panel_focus = None;
        self.plugins.editor_focus = Some(slot.clone());
        let open = self
            .plugins
            .completion
            .as_ref()
            .is_some_and(|c| c.editor == slot);
        match typed {
            Some(c) if c.is_alphanumeric() || c == '_' || c == '.' => {
                if open || c == '.' {
                    return self.plugin_complete(slot);
                }
                Task::none()
            }
            _ => {
                if open {
                    self.plugins.completion = None;
                }
                Task::none()
            }
        }
    }

    /// Asks the plugin what may come at the cursor of one of its editors.
    pub(super) fn plugin_complete(&mut self, slot: GridKey) -> Task<Message> {
        let completes = self
            .plugins
            .views
            .get(&slot.0)
            .is_some_and(|root| editor_completes(root, &slot.1));
        let Some((content, _)) = self.plugins.editors.get(&slot).filter(|_| completes) else {
            return Task::none();
        };
        let text = content.text();
        let cursor = content.cursor().position;
        let offset = offset_of(&text, cursor.line, cursor.column);
        let params = CompletionParams {
            panel: slot.0 .1.clone(),
            editor: slot.1.clone(),
            text,
            offset,
        };
        let plugin = slot.0 .0.clone();
        Task::perform(
            async move {
                let answer =
                    host::request(&plugin, methods::COMPLETION, params, COMPLETION_WAIT).await?;
                serde_json::from_value::<Completions>(answer).map_err(|e| e.to_string())
            },
            move |result| Message::Plugin(PluginMsg::Completed(slot.clone(), offset, result)),
        )
    }

    pub(super) fn plugin_completed(
        &mut self,
        slot: GridKey,
        at: usize,
        result: Result<Completions, String>,
    ) {
        let Some((content, _)) = self.plugins.editors.get(&slot) else {
            return;
        };
        let text = content.text();
        let cursor = content.cursor().position;
        // The cursor moved on since: the answer is old.
        if offset_of(&text, cursor.line, cursor.column) != at {
            return;
        }
        self.plugins.completion = match result {
            Ok(found) if !found.items.is_empty() => Some(NodeCompletion {
                from: found.from.unwrap_or_else(|| word_start(&text, at)),
                editor: slot,
                items: found.items,
                at,
                picked: 0,
            }),
            _ => None,
        };
    }

    /// The keys of the list of completions of an editor of a panel.
    pub(super) fn plugin_completion_key(&mut self, key: &str) -> Option<Task<Message>> {
        let open = self.plugins.completion.as_mut()?;
        let n = open.items.len();
        match key {
            "ArrowDown" => open.picked = (open.picked + 1) % n,
            "ArrowUp" => open.picked = (open.picked + n - 1) % n,
            "Escape" => self.plugins.completion = None,
            "Enter" | "Tab" => {
                let picked = open.picked;
                return Some(self.plugin_completion_accept(picked));
            }
            _ => return None,
        }
        Some(Task::none())
    }

    /// Puts a completion in place of the word at the cursor.
    pub(super) fn plugin_completion_accept(&mut self, index: usize) -> Task<Message> {
        let Some(open) = self.plugins.completion.take() else {
            return Task::none();
        };
        let Some(item) = open.items.get(index) else {
            return Task::none();
        };
        let Some((content, _)) = self.plugins.editors.get_mut(&open.editor) else {
            return Task::none();
        };
        let text = content.text();
        let chars: Vec<char> = text.chars().collect();
        let insert = item.insert.clone().unwrap_or_else(|| item.label.clone());
        let from = open.from.min(chars.len());
        let to = open.at.clamp(from, chars.len());
        let new: String = chars[..from]
            .iter()
            .chain(insert.chars().collect::<Vec<_>>().iter())
            .chain(chars[to..].iter())
            .collect();
        *content = iced::widget::text_editor::Content::with_text(&new);
        let (line, column) = position_of(&new, from + insert.chars().count());
        content.move_to(iced::advanced::text::editor::Cursor {
            position: iced::advanced::text::editor::Position { line, column },
            selection: None,
        });
        host::notify(
            &open.editor.0 .0,
            methods::UI_EVENT,
            UiEvent {
                panel: open.editor.0 .1.clone(),
                action: open.editor.1.clone(),
                value: Some(serde_json::Value::String(new)),
            },
        );
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::host::Incoming;
    use ardoise_plugin::protocol::Message as Rpc;

    fn app_with_panel(root: serde_json::Value) -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "test.grid", "name": "Grid", "version": "1.0.0", "api": 1, "command": ["bin/none"],
                "contributes": {"panels": [{"id": "p", "title": "P", "location": "bottom"}]}}"#,
        )
        .unwrap();
        app.plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(Incoming::Rpc(
            "test.grid".into(),
            Rpc::notification(
                methods::RENDER,
                serde_json::json!({"panel": "p", "root": root}),
            ),
        ))));
        app
    }

    fn key() -> GridKey {
        (
            ("test.grid".to_string(), "p".to_string()),
            "rows".to_string(),
        )
    }

    #[test]
    fn a_grid_of_a_panel_takes_the_keyboard_once_clicked_and_its_columns_are_dragged_wider() {
        let mut app = app_with_panel(serde_json::json!({
            "type": "grid", "id": "rows",
            "columns": [{"name": "id"}, {"name": "name", "width": 100.0}],
            "rows": [{"cells": [{"text": "1"}, {"text": "a"}]}, {"cells": [{"text": "2"}, {"null": true, "text": ""}]}],
            "selection": {"anchor": [0, 0], "cursor": [0, 0]}
        }));
        assert!(
            app.plugin_grid_key("ArrowDown", false, false, false)
                .is_none(),
            "not focused yet"
        );
        let _ = app.update(Message::Plugin(PluginMsg::Grid(
            "test.grid".into(),
            "p".into(),
            "rows".into(),
            GridMsg::Press(1, 1),
        )));
        assert_eq!(app.plugins.grid_focus, Some(key()));
        assert!(app
            .plugin_grid_key("ArrowDown", false, false, false)
            .is_some());
        assert!(app.plugin_grid_key("q", false, false, false).is_none());
        // The edge of "name" dragged 40 pixels to the right.
        let _ = app.update(Message::Plugin(PluginMsg::Grid(
            "test.grid".into(),
            "p".into(),
            "rows".into(),
            GridMsg::ResizeStart(1),
        )));
        assert!(app.plugin_grid_resizing());
        let _ = app.update(Message::Plugin(PluginMsg::GridResize(300.0)));
        let _ = app.update(Message::Plugin(PluginMsg::GridResize(340.0)));
        let _ = app.update(Message::Plugin(PluginMsg::GridResizeEnd));
        assert!(!app.plugin_grid_resizing());
        assert_eq!(app.plugins.grids[&key()].widths.get(&1), Some(&140.0));
        // Back in the editor: the keyboard is its own.
        let _ = app.update(Message::EditorAction(
            iced::widget::text_editor::Action::Click(iced::Point::ORIGIN),
        ));
        assert!(app.plugins.grid_focus.is_none());
    }

    #[test]
    fn a_key_of_the_menu_of_a_grid_does_its_item_and_a_key_of_a_button_presses_it() {
        let mut app = app_with_panel(serde_json::json!({
            "type": "column", "children": [
                {"type": "button", "label": "Copy SQL", "action": "copy_sql", "key": "Ctrl+C"},
                {"type": "input", "id": "filter", "value": ""},
                {"type": "grid", "id": "rows",
                 "columns": [{"name": "id"}],
                 "rows": [{"cells": [{"text": "1"}]}, {"cells": [{"text": "2"}]}],
                 "selection": {"anchor": [1, 0], "cursor": [1, 0]},
                 "menu": [{"label": "Form", "action": "form", "key": "F4"}]}
            ]
        }));
        // Nothing clicked in the panel: its keys are not for it.
        assert!(app.plugin_panel_key("c", true, false, false).is_none());
        let _ = app.update(Message::Plugin(PluginMsg::Ui(
            "test.grid".into(),
            "p".into(),
            "other".into(),
            None,
        )));
        assert!(app.plugin_panel_key("c", true, false, false).is_some());
        assert!(app.plugin_panel_key("c", false, false, false).is_none());
        // Typing in a field: Ctrl+C is the field's.
        let _ = app.update(Message::Plugin(PluginMsg::Input(
            "test.grid".into(),
            "p".into(),
            "filter".into(),
            "x".into(),
        )));
        assert!(app.plugin_panel_key("c", true, false, false).is_none());
        // F4 on the grid: the item of its menu, on the cell of the cursor.
        let _ = app.update(Message::Plugin(PluginMsg::Grid(
            "test.grid".into(),
            "p".into(),
            "rows".into(),
            GridMsg::Press(1, 0),
        )));
        let grid = app.grid_node(&key()).unwrap();
        assert_eq!(
            grid::menu_key(grid, "F4", false, false, false),
            Some(("form".to_string(), 1, 0))
        );
        assert_eq!(grid::menu_key(grid, "F4", true, false, false), None);
        assert!(app.plugin_grid_key("F4", false, false, false).is_some());
        // The editor again: no panel has the keyboard.
        let _ = app.update(Message::Plugin(PluginMsg::Ui(
            "test.grid".into(),
            "p".into(),
            "other".into(),
            None,
        )));
        let _ = app.update(Message::EditorAction(
            iced::widget::text_editor::Action::Click(iced::Point::ORIGIN),
        ));
        assert!(app.plugins.panel_focus.is_none());
    }

    #[test]
    fn a_file_a_plugin_brought_says_where_it_comes_from() {
        let mut app = app_with_panel(serde_json::json!({"type": "column", "children": []}));
        let path = PathBuf::from("/tmp/remote/web/index.html");
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(Incoming::Rpc(
            "test.grid".into(),
            Rpc::notification(
                methods::SET_ORIGIN,
                serde_json::json!({"path": path, "origin": "web:/srv/index.html"}),
            ),
        ))));
        assert_eq!(
            app.plugins.origins.get(&path).map(String::as_str),
            Some("web:/srv/index.html")
        );
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(Incoming::Rpc(
            "test.grid".into(),
            Rpc::notification(methods::SET_ORIGIN, serde_json::json!({"path": path})),
        ))));
        assert!(app.plugins.origins.is_empty());
    }

    #[test]
    fn the_lint_page_is_there_with_the_lint_plugin_and_shows_its_settings() {
        use crate::preferences::{PreferencesMessage, PrefsPage};
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        assert!(!PrefsPage::Lint.shown(&app.plugins), "no plugin, no page");
        // Without it, "Lint settings" lead to the plugins.
        let _ = app.update(Message::OpenLintSettings);
        assert_eq!(app.preferences.page, PrefsPage::Plugins);
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "test.lint", "name": "Lint", "version": "1.0.0", "api": 1, "command": ["bin/none"],
                "contributes": {"linter": {"extensions": ["js"]},
                "panels": [{"id": "settings", "title": "Linters", "location": "settings"}]}}"#,
        )
        .unwrap();
        app.plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
        assert!(PrefsPage::Lint.shown(&app.plugins));
        assert!(PrefsPage::Editor.shown(&app.plugins));
        // The Lint page opens the settings of the plugin, not the list.
        let _ = app.update(Message::Preferences(PreferencesMessage::PageSelected(
            PrefsPage::Lint,
        )));
        assert_eq!(app.preferences.page, PrefsPage::Lint);
        assert_eq!(
            app.plugins.settings_open,
            Some(("test.lint".to_string(), "settings".to_string()))
        );
        // Again: they stay open (not toggled shut).
        let _ = app.update(Message::OpenLintSettings);
        assert!(app.plugins.settings_open.is_some());
    }

    #[test]
    fn checking_for_an_update_of_the_app_updates_the_plugins_too() {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "test.old", "name": "Old", "version": "1.0.0", "api": 1, "command": ["bin/none"]}"#,
        )
        .unwrap();
        app.plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: crate::plugins::store::Origin {
                repository: Some("someone/plugin-old".into()),
                official: false,
                linked: None,
            },
        });
        let newer = || {
            Message::Plugin(PluginMsg::UpdatesChecked(vec![(
                "test.old".to_string(),
                Ok("1.1.0".to_string()),
            )]))
        };
        // Checked from the list of the plugins: the update is offered only.
        let _ = app.update(newer());
        assert!(app.plugins.updates.contains_key("test.old"));
        assert!(!app.plugins.busy.contains("test.old"));
        // Checked with the app: it installs.
        let _ = app.update(Message::CheckForUpdate);
        assert!(app.plugins.update_all && app.plugins.checking_updates);
        let _ = app.update(newer());
        assert!(app.plugins.busy.contains("test.old"), "being updated");
        assert!(!app.plugins.update_all);
    }

    #[test]
    fn a_plugin_is_uninstalled_with_its_data_only_when_asked() {
        use crate::app::ActiveOverlay;
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let add = |app: &mut ArdoiseApp, id: &str| {
            let manifest = ardoise_plugin::manifest::Manifest::parse(&format!(
                r#"{{"id": "{id}", "name": "Kept", "version": "1.0.0", "api": 1, "command": ["bin/none"]}}"#
            ))
            .unwrap();
            app.plugins.installed.push(crate::plugins::Installed {
                manifest,
                dir: PathBuf::from("/nowhere"),
                origin: Default::default(),
            });
            let data = crate::plugins::store::data_dir_of(id).unwrap();
            std::fs::write(data.join("settings.json"), "{}").unwrap();
            data
        };
        // The question comes first, the data unticked.
        let data = add(&mut app, "test.keeps");
        let _ = app.update(Message::Plugin(PluginMsg::AskUninstall(
            "test.keeps".into(),
        )));
        assert_eq!(
            app.overlay,
            ActiveOverlay::ConfirmPluginUninstall("test.keeps".into(), false)
        );
        let _ = app.update(Message::Plugin(PluginMsg::Uninstall("test.keeps".into())));
        assert!(app.plugins.get("test.keeps").is_none());
        assert!(
            data.join("settings.json").is_file(),
            "kept for a later install"
        );
        // With its data: the folder goes too.
        let data = add(&mut app, "test.forgets");
        let _ = app.update(Message::Plugin(PluginMsg::AskUninstall(
            "test.forgets".into(),
        )));
        let _ = app.update(Message::Plugin(PluginMsg::UninstallDataToggled(true)));
        assert_eq!(
            app.overlay,
            ActiveOverlay::ConfirmPluginUninstall("test.forgets".into(), true)
        );
        // It cannot start here: nothing outside of its folder to forget.
        let _ = app.update(Message::Plugin(PluginMsg::UninstallWithData(
            "test.forgets".into(),
        )));
        assert!(app.plugins.get("test.forgets").is_none());
        assert!(!data.exists(), "its data deleted");
        assert!(!app.plugins.busy.contains("test.forgets"));
    }

    #[test]
    fn git_and_the_terminal_are_offered_only_once_their_plugin_is_installed() {
        use crate::config::ShortcutTarget;
        use crate::palette::PaletteCommand;
        use crate::preferences::shortcut_shown;
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let offered = |app: &ArdoiseApp| {
            let tools = app.plugins.tools();
            let palette: Vec<PaletteCommand> =
                crate::palette::entries(&app.config, true, true, tools)
                    .into_iter()
                    .map(|e| e.command)
                    .collect();
            (
                shortcut_shown(ShortcutTarget::GitCommit, tools),
                shortcut_shown(ShortcutTarget::ToggleTerminal, tools),
                palette.contains(&PaletteCommand::ToggleGit),
                palette.contains(&PaletteCommand::ToggleTerminal),
            )
        };
        assert_eq!(offered(&app), (false, false, false, false));
        assert!(shortcut_shown(ShortcutTarget::Undo, app.plugins.tools()));
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "ardoise.git", "name": "Git", "version": "1.0.0", "api": 1, "command": ["bin/none"]}"#,
        )
        .unwrap();
        app.plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
        assert_eq!(offered(&app), (true, false, true, false));
    }

    #[test]
    fn a_plugin_with_settings_has_a_page_of_its_own_in_the_preferences() {
        use crate::preferences::{PreferencesMessage, PrefsPage};
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let add = |app: &mut ArdoiseApp, json: &str| {
            let manifest = ardoise_plugin::manifest::Manifest::parse(json).unwrap();
            app.plugins.installed.push(crate::plugins::Installed {
                manifest,
                dir: PathBuf::from("/nowhere"),
                origin: Default::default(),
            });
        };
        assert!(app.plugins.settings_pages().is_empty());
        add(
            &mut app,
            r#"{"id": "acme.zebra", "name": "Zebra", "version": "1.0.0", "api": 1, "command": ["bin/none"],
                "contributes": {"panels": [{"id": "settings", "title": "Zebra", "location": "settings"}]}}"#,
        );
        add(
            &mut app,
            r#"{"id": "acme.alpha", "name": "alpha", "version": "1.0.0", "api": 1, "command": ["bin/none"],
                "contributes": {"panels": [{"id": "settings", "title": "Alpha", "location": "settings"}]}}"#,
        );
        // No settings, no page; the linter has the Lint page instead.
        add(
            &mut app,
            r#"{"id": "acme.plain", "name": "Plain", "version": "1.0.0", "api": 1, "command": ["bin/none"]}"#,
        );
        add(
            &mut app,
            r#"{"id": "acme.lint", "name": "Lint", "version": "1.0.0", "api": 1, "command": ["bin/none"],
                "contributes": {"linter": {"extensions": ["js"]},
                "panels": [{"id": "settings", "title": "Linters", "location": "settings"}]}}"#,
        );
        let names: Vec<String> = app
            .plugins
            .settings_pages()
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert_eq!(names, vec!["alpha", "Zebra"], "by name, whatever the case");
        // Its page opens its settings (and closes the ones that were open).
        let _ = app.update(Message::Preferences(
            PreferencesMessage::PluginPageSelected("acme.zebra".into()),
        ));
        assert_eq!(app.preferences.page, PrefsPage::PluginSettings);
        assert_eq!(app.preferences.plugin_page.as_deref(), Some("acme.zebra"));
        assert_eq!(
            app.plugins.settings_open,
            Some(("acme.zebra".to_string(), "settings".to_string()))
        );
        let _ = app.update(Message::Preferences(
            PreferencesMessage::PluginPageSelected("acme.alpha".into()),
        ));
        assert_eq!(
            app.plugins.settings_open,
            Some(("acme.alpha".to_string(), "settings".to_string()))
        );
        // Removed, it has no page any more.
        app.plugins.installed.retain(|p| p.id() != "acme.alpha");
        let ids: Vec<String> = app
            .plugins
            .settings_pages()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, vec!["acme.zebra"]);
    }

    #[test]
    fn the_rows_scroll_to_a_cursor_moved_out_of_view() {
        let rows: Vec<_> = (0..50)
            .map(|i| serde_json::json!({"cells": [{"text": i.to_string()}]}))
            .collect();
        let grid = |row: usize| {
            serde_json::json!({
                "type": "grid", "id": "rows", "columns": [{"name": "n"}], "rows": rows,
                "selection": {"anchor": [row, 0], "cursor": [row, 0]}
            })
        };
        let mut app = app_with_panel(grid(0));
        app.plugins.grids.entry(key()).or_default().scroll = (0.0, 10.0 * grid::ROW_HEIGHT);
        let _ = app.update(Message::Plugin(PluginMsg::Incoming(Incoming::Rpc(
            "test.grid".into(),
            Rpc::notification(
                methods::RENDER,
                serde_json::json!({"panel": "p", "root": grid(30)}),
            ),
        ))));
        let view = &app.plugins.grids[&key()];
        assert_eq!(view.cursor, Some((30, 0)));
        assert_eq!(view.scroll.0, 21.0 * grid::ROW_HEIGHT);
    }

    #[test]
    fn a_completion_of_the_plugin_takes_the_place_of_the_word_at_the_cursor() {
        let mut app = app_with_panel(serde_json::json!({
            "type": "editor", "id": "sql", "value": "select na", "language": "sql", "completion": true
        }));
        let slot = (
            ("test.grid".to_string(), "p".to_string()),
            "sql".to_string(),
        );
        {
            let (content, _) = app.plugins.editors.get_mut(&slot).expect("its editor");
            content.perform(iced::widget::text_editor::Action::Move(
                iced::widget::text_editor::Motion::DocumentEnd,
            ));
        }
        app.plugin_completed(
            slot.clone(),
            9,
            Ok(Completions {
                items: vec![
                    ardoise_plugin::protocol::CompletionItem {
                        label: "name".into(),
                        ..Default::default()
                    },
                    ardoise_plugin::protocol::CompletionItem {
                        label: "nature".into(),
                        insert: Some("nature AS n".into()),
                        detail: "column".into(),
                    },
                ],
                from: None,
            }),
        );
        let open = app.plugins.completion.as_ref().expect("a list");
        assert_eq!(open.from, 7);
        assert!(app.plugin_completion_key("ArrowDown").is_some());
        assert!(app.plugin_completion_key("Enter").is_some());
        assert!(app.plugins.completion.is_none());
        let (content, _) = &app.plugins.editors[&slot];
        assert_eq!(content.text(), "select nature AS n");
        assert_eq!(content.cursor().position.column, 18);
        // An old answer (the cursor moved on) is dropped.
        app.plugin_completed(slot, 3, Ok(Completions::default()));
        assert!(app.plugins.completion.is_none());
    }

    #[test]
    fn a_cursor_is_counted_in_characters_and_found_back() {
        let text = "select é\nfrom t";
        assert_eq!(offset_of(text, 1, 4), 13);
        assert_eq!(position_of(text, 13), (1, 4));
        assert_eq!(position_of(text, 8), (0, 8));
        assert_eq!(word_start("from us", 7), 5);
        assert_eq!(word_start("a.b_c", 5), 2);
    }
}
