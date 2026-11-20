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

/// What the menus depend on.
pub struct MenuState<'a> {
    pub config: &'a Config,
    pub has_formatter: bool,
    pub lang_picker_enabled: bool,
    pub bookmarks: BookmarkState,
    pub tools: crate::plugins::Tools,
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
        item(
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
        item(t!("menu.file_save"), Some(&sh.save_file), Message::SaveFile),
        item(
            t!("menu.file_save_as"),
            Some(&sh.save_as),
            Message::SaveFileAs,
        ),
        item(
            t!("menu.file_close"),
            Some(&sh.close_file),
            Message::CloseFile,
        ),
        item(t!("menu.file_close_others"), None, Message::CloseOtherTabs),
        item(t!("menu.file_close_all"), None, Message::CloseTabs(None)),
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
    let marks = state.bookmarks;
    let line = sub(
        SubMenu::EditLine,
        t!("menu.sub_line"),
        vec![
            item(
                t!("menu.edit_duplicate_line"),
                Some(&sh.duplicate_line),
                Message::DuplicateLine,
            ),
            item(
                t!("menu.edit_move_line_up"),
                Some(&sh.move_line_up),
                Message::MoveLineUp,
            ),
            item(
                t!("menu.edit_move_line_down"),
                Some(&sh.move_line_down),
                Message::MoveLineDown,
            ),
            item(
                t!("menu.edit_delete_line"),
                Some(&sh.delete_line),
                Message::DeleteLine,
            ),
            item(
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
            item(t!("menu.lint_check"), None, Message::LintNow),
            item(t!("menu.lint_fix"), None, Message::LintFixAll),
            item(t!("menu.lint_next"), None, Message::LintNext(true)),
            item(t!("menu.lint_prev"), None, Message::LintNext(false)),
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
            item(toggle, Some(&sh.toggle_bookmark), Message::ToggleBookmark),
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
            item(
                t!("menu.fold_toggle"),
                Some(&sh.toggle_fold),
                Message::ToggleFold,
            ),
            item(t!("menu.fold_all"), None, Message::FoldAll),
            item(t!("menu.unfold_all"), None, Message::UnfoldAll),
        ],
    );
    vec![
        item(t!("menu.edit_undo"), Some(&sh.undo), Message::Undo),
        item(t!("menu.edit_redo"), Some(&sh.redo), Message::Redo),
        Entry::Separator,
        item(
            t!("menu.edit_select_all"),
            Some(&sh.select_all),
            Message::SelectAll,
        ),
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
        Entry::Separator,
        item(
            t!("menu.edit_toggle_comment"),
            Some(&sh.toggle_comment),
            Message::ToggleComment,
        ),
        line,
        problems,
        bookmarks,
        fold,
        Entry::Separator,
        item(t!("menu.nav_back"), Some(&sh.nav_back), Message::NavBack),
        item(
            t!("menu.nav_forward"),
            Some(&sh.nav_forward),
            Message::NavForward,
        ),
        Entry::Separator,
        item(
            t!("shortcuts.complete"),
            Some(&sh.complete),
            Message::CompletionInvoke,
        ),
        item(
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

fn view(state: &MenuState) -> Vec<Entry> {
    let config = state.config;
    let sh = &config.shortcuts;
    let mut panel = vec![
        item(
            t!("menu.view_sidebar"),
            Some(&sh.toggle_sidebar),
            Message::ToggleSidebar,
        ),
        item(t!("menu.view_split_close"), None, Message::CloseSplit),
    ];
    // The panels of the plugins, once the plugin is installed.
    if state.tools.git || state.tools.terminal {
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
                item(t!("menu.view_split_right"), None, Message::Split(true)),
                item(t!("menu.view_split_down"), None, Message::Split(false)),
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
        let edit = labels(&entries(TopMenu::Edit, &s, false));
        for moved in [
            t!("shortcuts.goto_file"),
            t!("shortcuts.find_in_files"),
            t!("shortcuts.replace_in_files"),
        ] {
            assert!(!file.contains(&moved.to_string()), "{moved} left File");
            assert!(edit.contains(&moved.to_string()), "{moved} is in Edit");
        }
        assert!(file.contains(&t!("shortcuts.reopen_tab").to_string()));
    }

    #[test]
    fn the_edit_menu_groups_lines_problems_bookmarks_and_folding_in_submenus() {
        let config = Config::default();
        let s = state(&config);
        let edit = entries(TopMenu::Edit, &s, false);
        let top = labels(&edit);
        for group in ["sub_line", "sub_problems", "sub_bookmarks", "sub_fold"] {
            let key = format!("menu.{group}");
            let title = t!(&key).to_string();
            assert!(top.contains(&format!("> {title}")), "{group}");
        }
        // None of what is in them is left at the top.
        for gone in [
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
                t!("menu.view_split_down").to_string()
            ]
        );
        assert_eq!(
            labels(submenu(&view, SubMenu::ViewPanel)),
            vec![
                t!("menu.view_sidebar").to_string(),
                t!("menu.view_split_close").to_string()
            ],
            "no plugin, no entry of a plugin"
        );
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
