//! What the menus hold, said once: the menu bar of the window and the system
//! menu bar of macOS (`native_menu`) both draw it. The submenus group what
//! goes together (a line, the problems, the bookmarks…) so that no menu is
//! a long list.

use std::path::PathBuf;

use crate::app::{Message, TopMenu};
use crate::config::{Config, ShortcutConfig};
use crate::menu_bar::BookmarkState;
use crate::preferences::PreferencesMessage;

/// The submenus that can be open (one at a time, beside their menu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubMenu {
    Recents,
    ViewPanel,
    ViewSplit,
    EditFind,
    EditLine,
    EditProblems,
    EditBookmarks,
    EditFold,
}

/// An entry of a menu.
#[derive(Debug, Clone)]
pub enum Entry {
    Item {
        label: String,
        /// The shortcut shown on the right (the window's menus only).
        shortcut: Option<String>,
        enabled: bool,
        message: Message,
    },
    Separator,
    Sub {
        id: SubMenu,
        label: String,
        enabled: bool,
        entries: Vec<Entry>,
    },
}

/// What the entries of the menus need to be on: an entry that cannot do
/// anything now is off, and wakes up when it can.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditorState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub can_back: bool,
    pub can_forward: bool,
    /// A tab was closed, and can be brought back.
    pub can_reopen: bool,
    /// More than one tab is open.
    pub several_tabs: bool,
    /// The file in front can be checked for problems.
    pub can_check: bool,
    /// The file in front has problems shown.
    pub has_problems: bool,
    pub fold: crate::code_editor::FoldState,
    /// There is code to complete (or a panel editor asking the plugin).
    pub can_complete: bool,
    /// There is code, and a language server to ask about the parameters.
    pub can_hint: bool,
    /// A file (or a tab) is in front.
    pub has_file: bool,
    /// The text can be changed now (not blocked by a popin, the
    /// preferences, a preview or a file too large).
    pub can_edit: bool,
    /// The language of the file has a syntax of comments.
    pub can_comment: bool,
    /// The lines the cursor is on are the first / the last of the text.
    pub first_line: bool,
    pub last_line: bool,
    /// Some text is selected.
    pub has_selection: bool,
    /// Some open file has unsaved changes.
    pub any_modified: bool,
    /// At least one tab is open.
    pub has_tabs: bool,
    /// The file in front can be shown in a second pane.
    pub can_split: bool,
}

/// What the menus depend on.
pub struct MenuState<'a> {
    pub config: &'a Config,
    pub has_formatter: bool,
    pub lang_picker_enabled: bool,
    pub bookmarks: BookmarkState,
    pub tools: crate::plugins::Tools,
    /// The editor is split: there is a pane to close.
    pub split: bool,
    /// The panels of the other plugins installed: (title, what shows it).
    pub panels: Vec<(String, Message)>,
    pub editor: EditorState,
}

fn item(label: impl Into<String>, shortcut: Option<&ShortcutConfig>, message: Message) -> Entry {
    Entry::Item {
        label: label.into(),
        shortcut: shortcut.map(ShortcutConfig::display),
        enabled: true,
        message,
    }
}

fn item_if(
    enabled: bool,
    label: impl Into<String>,
    shortcut: Option<&ShortcutConfig>,
    message: Message,
) -> Entry {
    match item(label, shortcut, message) {
        Entry::Item {
            label,
            shortcut,
            message,
            ..
        } => Entry::Item {
            label,
            shortcut,
            enabled,
            message,
        },
        other => other,
    }
}

fn sub(id: SubMenu, label: impl Into<String>, entries: Vec<Entry>) -> Entry {
    Entry::Sub {
        id,
        label: label.into(),
        enabled: true,
        entries,
    }
}

/// The entries of a menu. `native`: the system menu bar of macOS, which
/// keeps Preferences, Quit and About in the application's own menu.
pub fn entries(menu: TopMenu, state: &MenuState, native: bool) -> Vec<Entry> {
    match menu {
        TopMenu::File => file(state, native),
        TopMenu::Edit => edit(state),
        TopMenu::View => view(state),
        TopMenu::Help => help(native),
    }
}

fn file(state: &MenuState, native: bool) -> Vec<Entry> {
    let sh = &state.config.shortcuts;
    let ed = state.editor;
    let recents: Vec<PathBuf> = state
        .config
        .recent_files
        .iter()
        .map(PathBuf::from)
        .collect();
    let mut recent_entries: Vec<Entry> = recents
        .iter()
        .zip(crate::app::unique_labels(&recents))
        .map(|(path, name)| item(name, None, Message::OpenRecentFile(path.clone())))
        .collect();
    recent_entries.push(Entry::Separator);
    recent_entries.push(item(
        t!("menu.file_recents_clear"),
        None,
        Message::ClearRecents,
    ));
    let mut entries = vec![
        item(t!("menu.file_new"), Some(&sh.new_file), Message::NewFile),
        item(t!("menu.file_open"), Some(&sh.open_file), Message::OpenFile),
        item(t!("menu.file_add_project"), None, Message::AddProject),
        item_if(
            ed.can_reopen,
            t!("shortcuts.reopen_tab"),
            Some(&sh.reopen_tab),
            Message::ReopenTab,
        ),
        Entry::Separator,
        Entry::Sub {
            id: SubMenu::Recents,
            label: t!("menu.file_recents").to_string(),
            enabled: !recents.is_empty(),
            entries: recent_entries,
        },
        Entry::Separator,
        item_if(
            ed.has_file,
            t!("menu.file_save"),
            Some(&sh.save_file),
            Message::SaveFile,
        ),
        item_if(
            ed.has_file,
            t!("menu.file_save_as"),
            Some(&sh.save_as),
            Message::SaveFileAs,
        ),
        // Only when some open file has unsaved changes.
        item_if(
            ed.any_modified,
            t!("menu.file_save_all"),
            None,
            Message::SaveAll,
        ),
        Entry::Separator,
        item_if(
            ed.has_file,
            t!("menu.file_close"),
            Some(&sh.close_file),
            Message::CloseFile,
        ),
        item_if(
            ed.several_tabs,
            t!("menu.file_close_others"),
            None,
            Message::CloseOtherTabs,
        ),
        item_if(
            ed.has_tabs,
            t!("menu.file_close_all"),
            None,
            Message::CloseTabs(None),
        ),
    ];
    if !native {
        entries.push(Entry::Separator);
        entries.push(item(
            t!("menu.file_preferences"),
            None,
            Message::TogglePreferences,
        ));
        entries.push(item(t!("menu.file_quit"), Some(&sh.quit), Message::Quit));
    }
    entries
}

fn edit(state: &MenuState) -> Vec<Entry> {
    let sh = &state.config.shortcuts;
    let ed = state.editor;
    let marks = state.bookmarks;
    let find = sub(
        SubMenu::EditFind,
        t!("menu.sub_find"),
        vec![
            item(t!("menu.edit_find"), Some(&sh.find), Message::ToggleSearch),
            item(
                t!("shortcuts.goto_file"),
                Some(&sh.goto_file),
                Message::OpenGotoFile,
            ),
            item(
                t!("shortcuts.find_in_files"),
                Some(&sh.find_in_files),
                Message::OpenFindInFiles,
            ),
            item(
                t!("shortcuts.replace_in_files"),
                Some(&sh.replace_in_files),
                Message::OpenReplaceInFiles,
            ),
        ],
    );
    let line = sub(
        SubMenu::EditLine,
        t!("menu.sub_line"),
        vec![
            item_if(
                ed.can_edit,
                t!("menu.edit_duplicate_line"),
                Some(&sh.duplicate_line),
                Message::DuplicateLine,
            ),
            item_if(
                ed.can_edit && !ed.first_line,
                t!("menu.edit_move_line_up"),
                Some(&sh.move_line_up),
                Message::MoveLineUp,
            ),
            item_if(
                ed.can_edit && !ed.last_line,
                t!("menu.edit_move_line_down"),
                Some(&sh.move_line_down),
                Message::MoveLineDown,
            ),
            item_if(
                ed.can_edit,
                t!("menu.edit_delete_line"),
                Some(&sh.delete_line),
                Message::DeleteLine,
            ),
            item_if(
                ed.has_file,
                t!("menu.edit_goto_line"),
                Some(&sh.goto_line),
                Message::OpenGotoLine,
            ),
        ],
    );
    let problems = sub(
        SubMenu::EditProblems,
        t!("menu.sub_problems"),
        vec![
            item_if(ed.can_check, t!("menu.lint_check"), None, Message::LintNow),
            item_if(
                ed.can_check && ed.has_problems,
                t!("menu.lint_fix"),
                None,
                Message::LintFixAll,
            ),
            item_if(
                ed.has_problems,
                t!("menu.lint_next"),
                None,
                Message::LintNext(true),
            ),
            item_if(
                ed.has_problems,
                t!("menu.lint_prev"),
                None,
                Message::LintNext(false),
            ),
        ],
    );
    let toggle = if marks.on_line {
        t!("menu.bookmark_remove")
    } else {
        t!("menu.bookmark_add")
    };
    let bookmarks = sub(
        SubMenu::EditBookmarks,
        t!("menu.sub_bookmarks"),
        vec![
            item_if(
                ed.has_file,
                toggle,
                Some(&sh.toggle_bookmark),
                Message::ToggleBookmark,
            ),
            item_if(
                marks.any,
                t!("menu.bookmark_clear"),
                None,
                Message::ClearBookmarks,
            ),
            item_if(
                marks.any,
                t!("menu.bookmark_next"),
                Some(&sh.next_bookmark),
                Message::NextBookmark,
            ),
            item_if(
                marks.any,
                t!("menu.bookmark_prev"),
                Some(&sh.prev_bookmark),
                Message::PrevBookmark,
            ),
        ],
    );
    let fold = sub(
        SubMenu::EditFold,
        t!("menu.sub_fold"),
        vec![
            item_if(
                ed.fold.here,
                t!("menu.fold_toggle"),
                Some(&sh.toggle_fold),
                Message::ToggleFold,
            ),
            item_if(ed.fold.any, t!("menu.fold_all"), None, Message::FoldAll),
            item_if(
                ed.fold.folded,
                t!("menu.unfold_all"),
                None,
                Message::UnfoldAll,
            ),
        ],
    );
    vec![
        item_if(
            ed.can_undo && ed.can_edit,
            t!("menu.edit_undo"),
            Some(&sh.undo),
            Message::Undo,
        ),
        item_if(
            ed.can_redo && ed.can_edit,
            t!("menu.edit_redo"),
            Some(&sh.redo),
            Message::Redo,
        ),
        Entry::Separator,
        item_if(
            ed.can_edit && ed.has_selection,
            t!("menu.edit_cut"),
            Some(&ShortcutConfig::ctrl("x")),
            Message::ContextCut,
        ),
        item_if(
            ed.has_file && ed.has_selection,
            t!("menu.edit_copy"),
            Some(&ShortcutConfig::ctrl("c")),
            Message::ContextCopy,
        ),
        item_if(
            ed.can_edit,
            t!("menu.edit_paste"),
            Some(&ShortcutConfig::ctrl("v")),
            Message::ContextPaste,
        ),
        Entry::Separator,
        item_if(
            ed.has_file,
            t!("menu.edit_select_all"),
            Some(&sh.select_all),
            Message::SelectAll,
        ),
        find,
        Entry::Separator,
        item_if(
            ed.can_edit && ed.can_comment,
            t!("menu.edit_toggle_comment"),
            Some(&sh.toggle_comment),
            Message::ToggleComment,
        ),
        line,
        problems,
        bookmarks,
        fold,
        Entry::Separator,
        item_if(
            ed.can_back,
            t!("menu.nav_back"),
            Some(&sh.nav_back),
            Message::NavBack,
        ),
        item_if(
            ed.can_forward,
            t!("menu.nav_forward"),
            Some(&sh.nav_forward),
            Message::NavForward,
        ),
        Entry::Separator,
        item_if(
            ed.can_complete,
            t!("shortcuts.complete"),
            Some(&sh.complete),
            Message::CompletionInvoke,
        ),
        item_if(
            ed.can_hint,
            t!("shortcuts.parameter_info"),
            Some(&sh.parameter_info),
            Message::HintInvoke,
        ),
        item_if(
            state.has_formatter,
            t!("menu.edit_format"),
            Some(&sh.format_code),
            Message::FormatFile,
        ),
    ]
}

/// An entry that switches a setting: it says what it will do (the label of
/// the state it goes to).
fn toggle_entry(
    on: bool,
    when_on: impl Into<String>,
    when_off: impl Into<String>,
    message: PreferencesMessage,
) -> Entry {
    item(
        if on { when_on.into() } else { when_off.into() },
        None,
        Message::Preferences(message),
    )
}

fn view(state: &MenuState) -> Vec<Entry> {
    let config = state.config;
    let sh = &config.shortcuts;
    let mut panel = vec![item(
        t!("menu.view_sidebar"),
        Some(&sh.toggle_sidebar),
        Message::ToggleSidebar,
    )];
    // The panels of the plugins, once the plugin is installed.
    if state.tools.git || state.tools.terminal || !state.panels.is_empty() {
        panel.push(Entry::Separator);
    }
    if state.tools.git {
        panel.push(item(
            t!("git.title"),
            Some(&sh.toggle_git),
            Message::ToggleGit,
        ));
    }
    if state.tools.terminal {
        panel.push(item(
            t!("terminal.title"),
            Some(&sh.toggle_terminal),
            Message::ToggleTerminal,
        ));
    }
    // The panels of every other plugin installed, as soon as it is there.
    for (title, message) in &state.panels {
        panel.push(item(title.clone(), None, message.clone()));
    }
    let numbers = if config.show_line_numbers {
        t!("menu.view_line_numbers_hide")
    } else {
        t!("menu.view_line_numbers_show")
    };
    vec![
        item(
            t!("menu.view_palette"),
            Some(&sh.command_palette),
            Message::OpenPalette,
        ),
        Entry::Separator,
        sub(SubMenu::ViewPanel, t!("menu.sub_panel"), panel),
        sub(
            SubMenu::ViewSplit,
            t!("menu.sub_split"),
            vec![
                item_if(
                    state.editor.can_split,
                    t!("menu.view_split_right"),
                    None,
                    Message::Split(true),
                ),
                item_if(
                    state.editor.can_split,
                    t!("menu.view_split_down"),
                    None,
                    Message::Split(false),
                ),
                // Only when the editor is split: else there is nothing to close.
                item_if(
                    state.split,
                    t!("menu.view_split_close"),
                    None,
                    Message::CloseSplit,
                ),
            ],
        ),
        Entry::Separator,
        item(
            numbers,
            None,
            Message::Preferences(PreferencesMessage::ShowLineNumbersToggled(
                !config.show_line_numbers,
            )),
        ),
        item_if(
            state.lang_picker_enabled,
            t!("menu.view_language"),
            None,
            Message::OpenLanguagePicker,
        ),
        toggle_entry(
            config.word_wrap,
            t!("menu.view_wrap_off"),
            t!("menu.view_wrap_on"),
            PreferencesMessage::WordWrapToggled(!config.word_wrap),
        ),
        toggle_entry(
            config.show_minimap,
            t!("menu.view_minimap_hide"),
            t!("menu.view_minimap_show"),
            PreferencesMessage::ShowMinimapToggled(!config.show_minimap),
        ),
        toggle_entry(
            config.show_whitespace,
            t!("menu.view_whitespace_hide"),
            t!("menu.view_whitespace_show"),
            PreferencesMessage::ShowWhitespaceToggled(!config.show_whitespace),
        ),
        toggle_entry(
            config.show_indent_guides,
            t!("menu.view_indent_guides_hide"),
            t!("menu.view_indent_guides_show"),
            PreferencesMessage::ShowIndentGuidesToggled(!config.show_indent_guides),
        ),
    ]
}

fn help(native: bool) -> Vec<Entry> {
    let mut entries = Vec::new();
    // On macOS, About is in the menu of the application.
    if !native {
        entries.push(item(t!("menu.help_about"), None, Message::About));
    }
    entries.push(item(
        t!("menu.help_changelog"),
        None,
        Message::OpenChangelog,
    ));
    entries.push(item(
        t!("menu.help_check_updates"),
        None,
        Message::CheckForUpdate,
    ));
    entries.push(Entry::Separator);
    entries.push(item(
        t!("menu.help_shortcuts"),
        None,
        Message::OpenShortcuts,
    ));
    entries.push(item(
        t!("menu.help_docs"),
        None,
        Message::OpenUrl("https://ardoise-editor.github.io/ardoise/"),
    ));
    entries.push(item(
        t!("menu.help_report"),
        None,
        Message::OpenUrl("https://github.com/ardoise-editor/ardoise/issues/new"),
    ));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(config: &Config) -> MenuState<'_> {
        MenuState {
            config,
            has_formatter: true,
            lang_picker_enabled: true,
            bookmarks: BookmarkState::default(),
            tools: crate::plugins::Tools::default(),
            split: false,
            panels: Vec::new(),
            editor: EditorState::default(),
        }
    }

    /// The labels of the items of a list of entries, `>` marking a submenu.
    fn labels(entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .filter_map(|e| match e {
                Entry::Item { label, .. } => Some(label.clone()),
                Entry::Sub { label, .. } => Some(format!("> {label}")),
                Entry::Separator => None,
            })
            .collect()
    }

    fn submenu(entries: &[Entry], id: SubMenu) -> &[Entry] {
        entries
            .iter()
            .find_map(|e| match e {
                Entry::Sub {
                    id: this, entries, ..
                } if *this == id => Some(entries.as_slice()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no {id:?}"))
    }

    #[test]
    fn the_file_menu_keeps_files_and_the_searches_by_name_are_in_edit() {
        let config = Config::default();
        let s = state(&config);
        let file = labels(&entries(TopMenu::File, &s, false));
        let edit_menu = entries(TopMenu::Edit, &s, false);
        let edit = labels(submenu(&edit_menu, SubMenu::EditFind));
        for moved in [
            t!("shortcuts.goto_file"),
            t!("shortcuts.find_in_files"),
            t!("shortcuts.replace_in_files"),
        ] {
            assert!(!file.contains(&moved.to_string()), "{moved} left File");
            assert!(edit.contains(&moved.to_string()), "{moved} is in Find");
        }
        assert!(file.contains(&t!("shortcuts.reopen_tab").to_string()));
    }

    #[test]
    fn the_edit_menu_groups_lines_problems_bookmarks_and_folding_in_submenus() {
        let config = Config::default();
        let s = state(&config);
        let edit = entries(TopMenu::Edit, &s, false);
        let top = labels(&edit);
        for group in [
            "sub_find",
            "sub_line",
            "sub_problems",
            "sub_bookmarks",
            "sub_fold",
        ] {
            let key = format!("menu.{group}");
            let title = t!(&key).to_string();
            assert!(top.contains(&format!("> {title}")), "{group}");
        }
        // None of what is in them is left at the top.
        for gone in [
            t!("menu.edit_find"),
            t!("shortcuts.goto_file"),
            t!("shortcuts.find_in_files"),
            t!("menu.edit_duplicate_line"),
            t!("menu.edit_goto_line"),
            t!("menu.lint_check"),
            t!("menu.bookmark_next"),
            t!("menu.fold_all"),
        ] {
            assert!(!top.contains(&gone.to_string()), "{gone}");
        }
        assert_eq!(
            labels(submenu(&edit, SubMenu::EditLine)),
            vec![
                t!("menu.edit_duplicate_line").to_string(),
                t!("menu.edit_move_line_up").to_string(),
                t!("menu.edit_move_line_down").to_string(),
                t!("menu.edit_delete_line").to_string(),
                t!("menu.edit_goto_line").to_string(),
            ]
        );
        assert_eq!(labels(submenu(&edit, SubMenu::EditFind)).len(), 4);
        // "Select all" stays on its own, above.
        assert!(top.contains(&t!("menu.edit_select_all").to_string()));
        assert_eq!(labels(submenu(&edit, SubMenu::EditProblems)).len(), 4);
        assert_eq!(labels(submenu(&edit, SubMenu::EditFold)).len(), 3);
    }

    #[test]
    fn the_bookmark_entries_follow_the_bookmarks_of_the_file() {
        let config = Config::default();
        let mut s = state(&config);
        let enabled = |s: &MenuState| -> Vec<(String, bool)> {
            submenu(&entries(TopMenu::Edit, s, false), SubMenu::EditBookmarks)
                .iter()
                .filter_map(|e| match e {
                    Entry::Item { label, enabled, .. } => Some((label.clone(), *enabled)),
                    _ => None,
                })
                .collect()
        };
        s.editor.has_file = true;
        let none = enabled(&s);
        assert_eq!(none[0], (t!("menu.bookmark_add").to_string(), true));
        assert!(
            none[1..].iter().all(|(_, on)| !on),
            "nothing to go to or clear"
        );
        s.bookmarks = BookmarkState {
            on_line: true,
            any: true,
        };
        let some = enabled(&s);
        assert_eq!(some[0], (t!("menu.bookmark_remove").to_string(), true));
        assert!(some.iter().all(|(_, on)| *on));
    }

    /// Whether the entry with this label is on, wherever it is in the menu.
    fn is_on(entries: &[Entry], label: &str) -> Option<bool> {
        entries.iter().find_map(|e| match e {
            Entry::Item {
                label: l, enabled, ..
            } if l == label => Some(*enabled),
            Entry::Sub {
                entries: inner,
                label: l,
                ..
            } => {
                if l == label {
                    None
                } else {
                    is_on(inner, label)
                }
            }
            _ => None,
        })
    }

    #[test]
    fn the_view_menu_toggles_the_display_and_splits_only_a_file_in_front() {
        let config = Config::default();
        let mut s = state(&config);
        let all = |s: &MenuState| entries(TopMenu::View, s, false);
        // No file: nothing to split.
        let split = |s: &MenuState| -> Vec<(String, bool)> {
            submenu(&all(s), SubMenu::ViewSplit)
                .iter()
                .filter_map(|e| match e {
                    Entry::Item { label, enabled, .. } => Some((label.clone(), *enabled)),
                    _ => None,
                })
                .collect()
        };
        assert!(!split(&s)[0].1 && !split(&s)[1].1);
        s.editor.can_split = true;
        assert!(split(&s)[0].1 && split(&s)[1].1);
        // The switches say what they will do, and flip.
        let shown = labels(&all(&state(&config)));
        for key in [
            "menu.view_line_numbers_hide",
            "menu.view_wrap_off",
            "menu.view_minimap_hide",
            "menu.view_indent_guides_hide",
            "menu.view_whitespace_show",
        ] {
            assert!(shown.contains(&t!(key).to_string()), "{key}");
        }
        let off = Config {
            word_wrap: false,
            show_minimap: false,
            ..Config::default()
        };
        let shown = labels(&all(&state(&off)));
        assert!(shown.contains(&t!("menu.view_wrap_on").to_string()));
        assert!(shown.contains(&t!("menu.view_minimap_show").to_string()));
    }

    #[test]
    fn the_help_menu_has_the_news_the_updates_the_shortcuts_the_docs_and_the_reports() {
        let config = Config::default();
        let help = entries(TopMenu::Help, &state(&config), false);
        let shown = labels(&help);
        for key in [
            "menu.help_about",
            "menu.help_changelog",
            "menu.help_check_updates",
            "menu.help_shortcuts",
            "menu.help_docs",
            "menu.help_report",
        ] {
            assert!(shown.contains(&t!(key).to_string()), "{key}");
        }
        // The links go to the project, over https.
        let urls: Vec<&str> = help
            .iter()
            .filter_map(|e| match e {
                Entry::Item {
                    message: Message::OpenUrl(url),
                    ..
                } => Some(*url),
                _ => None,
            })
            .collect();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().all(|u| u.starts_with("https://")));
    }

    #[test]
    fn an_entry_that_cannot_do_anything_yet_is_off_and_wakes_up_when_it_can() {
        let config = Config::default();
        let all = |s: &MenuState| -> Vec<Entry> {
            let mut all = entries(TopMenu::File, s, false);
            all.extend(entries(TopMenu::Edit, s, false));
            all
        };
        // A fresh editor: nothing to undo, to go back to, to check, to fold.
        let fresh = state(&config);
        let shown = all(&fresh);
        for key in [
            "menu.edit_undo",
            "menu.edit_redo",
            "menu.nav_back",
            "menu.nav_forward",
            "menu.lint_check",
            "menu.lint_fix",
            "menu.lint_next",
            "menu.lint_prev",
            "menu.fold_toggle",
            "menu.fold_all",
            "menu.unfold_all",
            "shortcuts.complete",
            "shortcuts.parameter_info",
            "menu.file_close_others",
            "shortcuts.reopen_tab",
            "menu.file_save",
            "menu.file_save_as",
            "menu.file_save_all",
            "menu.file_close",
            "menu.file_close_all",
            "menu.edit_cut",
            "menu.edit_copy",
            "menu.edit_paste",
            "menu.edit_select_all",
            "menu.edit_toggle_comment",
            "menu.edit_duplicate_line",
            "menu.edit_move_line_up",
            "menu.edit_move_line_down",
            "menu.edit_delete_line",
            "menu.edit_goto_line",
            "menu.bookmark_add",
        ] {
            let label = t!(key).to_string();
            assert_eq!(
                is_on(&shown, &label),
                Some(false),
                "{key} is off at the start"
            );
        }
        // What always works stays on.
        for key in ["menu.file_new", "menu.file_open", "menu.edit_find"] {
            assert_eq!(is_on(&shown, &t!(key)), Some(true), "{key}");
        }
        // Each wakes up with what it needs.
        let wake = |key: &str, edit: fn(&mut EditorState)| {
            let mut s = state(&config);
            edit(&mut s.editor);
            assert_eq!(is_on(&all(&s), &t!(key)), Some(true), "{key}");
        };
        wake("menu.edit_undo", |e| {
            e.can_undo = true;
            e.can_edit = true;
        });
        wake("menu.edit_redo", |e| {
            e.can_redo = true;
            e.can_edit = true;
        });
        for key in [
            "menu.file_save",
            "menu.file_save_as",
            "menu.file_close",
            "menu.edit_select_all",
            "menu.edit_goto_line",
            "menu.bookmark_add",
        ] {
            wake(key, |e| e.has_file = true);
        }
        wake("menu.file_save_all", |e| e.any_modified = true);
        wake("menu.file_close_all", |e| e.has_tabs = true);
        wake("menu.edit_cut", |e| {
            e.can_edit = true;
            e.has_selection = true;
        });
        wake("menu.edit_copy", |e| {
            e.has_file = true;
            e.has_selection = true;
        });
        wake("menu.edit_paste", |e| e.can_edit = true);
        wake("menu.edit_toggle_comment", |e| {
            e.can_edit = true;
            e.can_comment = true;
        });
        for key in [
            "menu.edit_duplicate_line",
            "menu.edit_delete_line",
            "menu.edit_move_line_up",
            "menu.edit_move_line_down",
        ] {
            wake(key, |e| e.can_edit = true);
        }
        wake("menu.nav_back", |e| e.can_back = true);
        wake("menu.nav_forward", |e| e.can_forward = true);
        wake("shortcuts.reopen_tab", |e| e.can_reopen = true);
        wake("menu.file_close_others", |e| e.several_tabs = true);
        wake("menu.lint_check", |e| e.can_check = true);
        wake("menu.lint_next", |e| e.has_problems = true);
        wake("menu.lint_prev", |e| e.has_problems = true);
        wake("menu.fold_toggle", |e| e.fold.here = true);
        wake("menu.fold_all", |e| e.fold.any = true);
        wake("menu.unfold_all", |e| e.fold.folded = true);
        wake("shortcuts.complete", |e| e.can_complete = true);
        wake("shortcuts.parameter_info", |e| e.can_hint = true);
        // The first line cannot go up, the last cannot go down.
        let mut ends = state(&config);
        ends.editor.can_edit = true;
        ends.editor.first_line = true;
        assert_eq!(
            is_on(&all(&ends), &t!("menu.edit_move_line_up")),
            Some(false)
        );
        assert_eq!(
            is_on(&all(&ends), &t!("menu.edit_move_line_down")),
            Some(true)
        );
        ends.editor.first_line = false;
        ends.editor.last_line = true;
        assert_eq!(
            is_on(&all(&ends), &t!("menu.edit_move_line_down")),
            Some(false)
        );
        // Fixing needs a file that can be checked and problems to fix.
        let mut fix = state(&config);
        fix.editor.has_problems = true;
        assert_eq!(is_on(&all(&fix), &t!("menu.lint_fix")), Some(false));
        fix.editor.can_check = true;
        assert_eq!(is_on(&all(&fix), &t!("menu.lint_fix")), Some(true));
    }

    #[test]
    fn the_view_menu_has_the_panel_and_split_submenus_and_no_theme_switch() {
        let config = Config::default();
        let s = state(&config);
        let view = entries(TopMenu::View, &s, false);
        let top = labels(&view);
        assert!(top.contains(&format!("> {}", t!("menu.sub_panel"))));
        assert!(top.contains(&format!("> {}", t!("menu.sub_split"))));
        for gone in [
            t!("toolbar.light"),
            t!("toolbar.dark"),
            t!("menu.view_sidebar"),
        ] {
            assert!(!top.contains(&gone.to_string()), "{gone}");
        }
        assert_eq!(
            labels(submenu(&view, SubMenu::ViewSplit)),
            vec![
                t!("menu.view_split_right").to_string(),
                t!("menu.view_split_down").to_string(),
                t!("menu.view_split_close").to_string()
            ]
        );
        assert_eq!(
            labels(submenu(&view, SubMenu::ViewPanel)),
            vec![t!("menu.view_sidebar").to_string()],
            "no plugin, no entry of a plugin; closing a split is in Split"
        );
        // Closing a split is off until the editor is split.
        let on = |state: &MenuState| -> bool {
            let view = entries(TopMenu::View, state, false);
            submenu(&view, SubMenu::ViewSplit).iter().any(|e| {
                matches!(e, Entry::Item { label, enabled: true, .. }
                    if label.as_str() == &*t!("menu.view_split_close"))
            })
        };
        // The panels of the plugins installed are in Panel, after the ones
        // with a shortcut.
        let mut with_panels = state(&config);
        with_panels.panels = vec![
            ("API".to_string(), Message::ToggleSidebar),
            ("Database".to_string(), Message::ToggleSidebar),
        ];
        let view_with = entries(TopMenu::View, &with_panels, false);
        let shown = labels(submenu(&view_with, SubMenu::ViewPanel));
        assert_eq!(shown.last().map(String::as_str), Some("Database"));
        assert!(shown.contains(&"API".to_string()));
        let mut split = state(&config);
        assert!(!on(&split));
        split.split = true;
        assert!(on(&split));
    }

    #[test]
    fn the_panels_of_the_plugins_join_the_panel_submenu_once_installed() {
        let config = Config::default();
        let mut s = state(&config);
        s.tools = crate::plugins::Tools {
            git: true,
            terminal: true,
        };
        let view = entries(TopMenu::View, &s, false);
        let panel = labels(submenu(&view, SubMenu::ViewPanel));
        assert!(panel.contains(&t!("git.title").to_string()));
        assert!(panel.contains(&t!("terminal.title").to_string()));
        // Not at the top of the menu.
        let top = labels(&view);
        assert!(!top.contains(&t!("git.title").to_string()));
    }

    #[test]
    fn the_system_menu_of_macos_leaves_what_the_application_menu_has() {
        let config = Config::default();
        let s = state(&config);
        let window = labels(&entries(TopMenu::File, &s, false));
        let native = labels(&entries(TopMenu::File, &s, true));
        for own in [t!("menu.file_preferences"), t!("menu.file_quit")] {
            assert!(window.contains(&own.to_string()) && !native.contains(&own.to_string()));
        }
        let about = t!("menu.help_about").to_string();
        assert!(labels(&entries(TopMenu::Help, &s, false)).contains(&about));
        assert!(!labels(&entries(TopMenu::Help, &s, true)).contains(&about));
    }

    #[test]
    fn the_recent_files_are_a_submenu_that_is_off_without_any() {
        let mut config = Config::default();
        let s = state(&config);
        let file = entries(TopMenu::File, &s, false);
        assert!(file.iter().any(|e| matches!(
            e,
            Entry::Sub {
                id: SubMenu::Recents,
                enabled: false,
                ..
            }
        )));
        config.recent_files = vec!["/tmp/a.txt".into(), "/tmp/b.txt".into()];
        let s = state(&config);
        let file = entries(TopMenu::File, &s, false);
        let recents = submenu(&file, SubMenu::Recents);
        // The files, then "Clear".
        assert_eq!(labels(recents).len(), 3);
        assert!(file.iter().any(|e| matches!(
            e,
            Entry::Sub {
                id: SubMenu::Recents,
                enabled: true,
                ..
            }
        )));
    }
}
