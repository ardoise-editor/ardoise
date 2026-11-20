use super::*;

#[cfg(test)]
mod short_dir_tests {
    use super::*;

    #[test]
    fn a_long_folder_keeps_its_end_within_the_limit() {
        let dir = Path::new("/very/long/folder/name")
            .join("x".repeat(80))
            .join("project");
        let shown = short_dir(&dir);
        assert_eq!(shown.chars().count(), MAX_DIR_CHARS);
        assert!(shown.starts_with('…'));
        assert!(shown.ends_with("/project/"));
    }

    #[test]
    fn a_short_folder_is_left_alone() {
        assert_eq!(short_dir(Path::new("/tmp/a")), "/tmp/a/");
    }
}

#[cfg(test)]
mod behavior_tests {
    use super::*;
    use text_editor::{Action, Edit};

    fn app() -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        app
    }

    #[test]
    fn what_comes_by_itself_or_from_passing_over_leaves_a_menu_open() {
        let mut app = app();
        let _ = app.handle(Message::ToggleMenu(TopMenu::Edit));
        assert_eq!(app.open_menu, Some(TopMenu::Edit));
        for message in [
            Message::TreeHover(PathBuf::from("/x")),
            Message::TreeScrolled(0.0, 100.0),
            Message::LintHover(None),
            Message::LintCardHover(false),
            Message::LintCloseCheck(0),
            Message::FinderFiles(Vec::new()),
            Message::FindResults(0, Ok(Vec::new())),
            Message::SymbolHover(None),
            Message::SymbolHoverLoaded(0, iced::Point::ORIGIN, Ok(None)),
            Message::IpcPoll,
            Message::FsChanged,
            Message::FileSaved(None, Err("x".into())),
            Message::FileReloaded(Err("x".into())),
            Message::LargeRefreshed(PathBuf::from("/x"), Err("x".into())),
        ] {
            let _ = app.update(message);
            assert_eq!(app.open_menu, Some(TopMenu::Edit));
        }
        // A real action closes it.
        let _ = app.update(Message::ToggleSidebar);
        assert_eq!(app.open_menu, None);
    }

    #[test]
    fn the_pointer_in_an_open_menu_does_not_reach_what_lies_beneath() {
        let mut app = app();
        let _ = app.handle(Message::ToggleMenu(TopMenu::Help));
        let anchor =
            iced::Rectangle::new(iced::Point::new(200.0, 8.0), iced::Size::new(50.0, 26.0));
        let _ = app.handle(Message::MenuBounds(TopMenu::Help, Some(anchor)));
        let mut ui =
            iced_test::Simulator::with_size(iced::Settings::default(), (1280.0, 800.0), app.view());
        // The card's padding, between its edge and the first entry: neither
        // an entry nor the area outside that closes the menu.
        ui.point_at(iced::Point::new(
            anchor.x + 2.0,
            anchor.y + anchor.height + 2.0,
        ));
        let _ = ui.simulate(iced_test::simulator::click());
        // Far below the card, over the editor: the click closes the menu,
        // and nothing else hears the pointer.
        ui.point_at(iced::Point::new(600.0, 500.0));
        let _ = ui.simulate(iced_test::simulator::click());
        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(
            messages
                .iter()
                .filter(|m| matches!(m, Message::CloseMenu))
                .count(),
            1,
            "{messages:?}"
        );
        assert!(
            messages.iter().all(|m| matches!(m, Message::CloseMenu)),
            "{messages:?}"
        );
    }

    #[test]
    fn a_submenu_opens_and_closes_with_the_menu() {
        use crate::menus::SubMenu;
        let mut app = app();
        let _ = app.handle(Message::ToggleMenu(TopMenu::File));
        let _ = app.handle(Message::SetSubmenu(Some(SubMenu::Recents)));
        assert_eq!(app.open_submenu, Some(SubMenu::Recents));
        // Pointing at another entry folds it away.
        let _ = app.handle(Message::SetSubmenu(None));
        assert_eq!(app.open_submenu, None);
        // Another submenu takes its place; changing the menu closes it.
        let _ = app.handle(Message::ToggleMenu(TopMenu::Edit));
        let _ = app.handle(Message::SetSubmenu(Some(SubMenu::EditLine)));
        assert_eq!(app.open_submenu, Some(SubMenu::EditLine));
        let _ = app.handle(Message::HoverMenu(TopMenu::View));
        assert_eq!(app.open_submenu, None);
        let _ = app.handle(Message::SetSubmenu(Some(SubMenu::ViewSplit)));
        let _ = app.handle(Message::ToggleMenu(TopMenu::View));
        assert_eq!(app.open_submenu, None);
    }

    fn type_text(app: &mut ArdoiseApp, text: &str) {
        for c in text.chars() {
            let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Insert(c))));
        }
    }

    fn text(app: &ArdoiseApp) -> String {
        crate::editor::buffer_text(&app.editor.content)
    }

    #[test]
    fn the_file_list_is_resized_by_dragging_its_edge() {
        let mut app = app();
        assert!(!app.sidebar_resizing);
        let _ = app.handle(Message::SidebarResizeStart);
        assert!(app.sidebar_resizing);
        // The list starts after the bar of panels and its hairline.
        let gap = super::ACTIVITY_WIDTH + 1.0;
        let _ = app.handle(Message::SidebarResize(300.0 + gap));
        assert_eq!(app.sidebar_width, 300.0);
        // Never thinner than the names need, nor wider than the window allows.
        let _ = app.handle(Message::SidebarResize(5.0));
        assert_eq!(app.sidebar_width, super::SIDEBAR_MIN);
        let _ = app.handle(Message::SidebarResize(1.0e6));
        assert!(app.sidebar_width <= super::SIDEBAR_MAX);
        assert!(app.sidebar_width <= app.config.window_width * 0.6 + 0.01);
        let _ = app.handle(Message::SidebarResize(300.0 + gap));
        let _ = app.handle(Message::SidebarResizeEnd);
        assert!(!app.sidebar_resizing);
        assert_eq!(app.config.sidebar_width, 300.0, "remembered");
    }

    #[test]
    fn the_bottom_panels_are_resized_by_dragging_its_top_edge() {
        let mut app = app();
        let start = app.panel_height;
        let _ = app.handle(Message::PanelResizeStart);
        assert!(app.panel_resizing);
        // The first move only anchors the drag; moving up makes it taller.
        let _ = app.handle(Message::PanelResize(500.0));
        assert_eq!(app.panel_height, start);
        let _ = app.handle(Message::PanelResize(450.0));
        assert_eq!(app.panel_height, start + 50.0);
        // Within the least and the most the window allows.
        let _ = app.handle(Message::PanelResize(1.0e6));
        assert_eq!(app.panel_height, super::PANEL_MIN_HEIGHT);
        let _ = app.handle(Message::PanelResize(-1.0e6));
        assert!(app.panel_height <= app.config.window_height * 0.85 + 0.01);
        let _ = app.handle(Message::PanelResizeEnd);
        assert!(!app.panel_resizing && app.panel_drag.is_none());
    }

    fn open_text(app: &mut ArdoiseApp, path: &Path, text: &str) {
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.to_path_buf(),
            text: text.into(),
            format: TextFormat::default(),
            stamp: None,
        })));
    }

    fn pane_files(app: &ArdoiseApp) -> Vec<PathBuf> {
        let split = app.split.as_ref().expect("split");
        split
            .layout_ids()
            .iter()
            .map(|id| split.panes[id].file.clone())
            .collect()
    }

    fn paste(text: &str) -> text_editor::Action {
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(text.into())))
    }

    #[test]
    fn the_editor_splits_in_many_panes_that_swap_on_focus() {
        let mut app = app();
        let (a, b) = (PathBuf::from("/s/a.txt"), PathBuf::from("/s/b.txt"));
        open_text(&mut app, &a, "alpha\n");
        // The same file in both panes: the new one is a copy that follows.
        let _ = app.update(Message::Split(true));
        assert_eq!(pane_files(&app), vec![a.clone(), a.clone()]);
        let _ = app.update(Message::EditorAction(paste("x")));
        assert_eq!(text(&app), "xalpha\n");
        let other = app.split.as_ref().unwrap().layout_ids()[1];
        assert_eq!(
            app.pane_content_mut(other).map(|c| c.text()),
            Some("xalpha\n".to_string())
        );
        // Another split, below: three panes in a tree.
        let _ = app.update(Message::Split(false));
        assert_eq!(app.split.as_ref().unwrap().panes.len(), 3);
        // Another file in the active pane: the others keep `a`, parked.
        open_text(&mut app, &b, "beta\n");
        assert_eq!(app.current_file.as_ref(), Some(&b));
        assert_eq!(pane_files(&app), vec![b.clone(), a.clone(), a.clone()]);
        // Clicking in another pane brings its file to the editor.
        let id = app.split.as_ref().unwrap().layout_ids()[2];
        let _ = app.update(Message::SplitFocus(id));
        assert_eq!(app.current_file.as_ref(), Some(&a));
        assert_eq!(app.split.as_ref().unwrap().active, id);
        assert_eq!(text(&app), "xalpha\n");
        // Closing the file of a pane takes the pane away, the split too once
        // one pane is left.
        let _ = app.update(Message::CloseOtherTabs);
        assert_eq!(pane_files(&app), vec![a.clone(), a.clone()]);
        // Closing the active pane gives the focus to another one.
        let _ = app.update(Message::CloseSplit);
        assert!(app.split.is_none());
        assert_eq!(app.current_file.as_ref(), Some(&a));
    }

    #[test]
    fn a_pane_that_is_not_the_first_can_be_split_too() {
        let mut app = app();
        open_text(&mut app, Path::new("/s/a.txt"), "alpha\n");
        let _ = app.update(Message::Split(true));
        let second = app.split.as_ref().unwrap().layout_ids()[1];
        let _ = app.update(Message::SplitFocus(second));
        let _ = app.update(Message::Split(true));
        assert_eq!(app.split.as_ref().unwrap().panes.len(), 3);
        let _ = app.update(Message::Split(false));
        assert_eq!(app.split.as_ref().unwrap().panes.len(), 4);
    }

    #[test]
    fn a_file_of_the_list_opens_in_a_new_pane() {
        let mut app = app();
        let (a, b) = (PathBuf::from("/s/a.txt"), PathBuf::from("/s/b.txt"));
        open_text(&mut app, &a, "alpha\n");
        open_text(&mut app, &b, "beta\n");
        let _ = app.update(Message::Sidebar(crate::sidebar::SidebarMessage::OpenFile(
            a.clone(),
        )));
        let _ = app.update(Message::SplitWith(b.clone(), false));
        assert_eq!(app.current_file.as_ref(), Some(&b));
        let split = app.split.as_ref().expect("split");
        assert_eq!(pane_files(&app), vec![a, b]);
        assert_ne!(split.layout_ids()[0], split.active);
    }

    #[test]
    fn a_scroll_stays_in_its_pane_and_a_click_takes_the_focus() {
        let mut app = app();
        let a = PathBuf::from("/s/a.txt");
        open_text(&mut app, &a, &"line\n".repeat(200));
        let _ = app.update(Message::Split(true));
        let (first, second) = {
            let ids = app.split.as_ref().unwrap().layout_ids();
            (ids[0], ids[1])
        };
        let _ = app.update(Message::SplitAction(
            second,
            text_editor::Action::Scroll { lines: 5 },
        ));
        assert_eq!(app.split.as_ref().unwrap().active, first, "no focus change");
        let _ = app.update(Message::SplitAction(
            second,
            text_editor::Action::Move(text_editor::Motion::Down),
        ));
        assert_eq!(app.split.as_ref().unwrap().active, second);
    }

    #[test]
    fn a_folder_added_becomes_a_project_that_is_kept_and_followed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        let mut app = app();
        let _ = app.update(Message::ProjectPicked(Some(dir.path().to_path_buf())));
        assert_eq!(app.projects.roots, vec![dir.path().to_path_buf()]);
        assert_eq!(
            app.config.projects, app.projects.roots,
            "kept in the settings"
        );
        // A folder of the tree opens, and the settings remember it.
        let src = dir.path().join("src");
        let _ = app.update(Message::ToggleProjectFolder(src.clone()));
        assert!(app.config.projects_open.contains(&src));
        assert!(app
            .projects
            .rows()
            .iter()
            .any(|r| r.entry.name == "main.rs"));
        // The same folder again, or none chosen: nothing changes.
        let _ = app.update(Message::ProjectPicked(Some(dir.path().to_path_buf())));
        let _ = app.update(Message::ProjectPicked(None));
        assert_eq!(app.projects.roots.len(), 1);
        // A file of the tree opens like any file of the list.
        let file = src.join("main.rs");
        let _ = app.update(Message::Sidebar(crate::sidebar::SidebarMessage::OpenFile(
            file.clone(),
        )));
        assert_eq!(app.pending_open.as_ref(), Some(&file));
        // A file of the list can be shown in the tree: its folders open.
        let _ = app.update(Message::ToggleProjectFolder(src.clone()));
        assert!(!app
            .projects
            .rows()
            .iter()
            .any(|r| r.entry.name == "main.rs"));
        let _ = app.update(Message::ShowInProjects(file.clone()));
        assert!(app
            .projects
            .rows()
            .iter()
            .any(|r| r.entry.name == "main.rs"));
        // What appears on the disk shows up by itself.
        std::fs::write(src.join("lib.rs"), "").unwrap();
        let _ = app.update(Message::RefreshProjects);
        assert!(app.projects.rows().iter().any(|r| r.entry.name == "lib.rs"));
        // The next run starts with the same projects, folders open as they were.
        let again = ArdoiseApp::blank(app.config.clone());
        assert_eq!(again.projects.roots, app.projects.roots);
        assert!(again
            .projects
            .rows()
            .iter()
            .any(|r| r.entry.name == "main.rs"));
        // Taking it out of the sidebar deletes nothing.
        let _ = app.update(Message::RemoveProject(dir.path().to_path_buf()));
        assert!(app.projects.roots.is_empty() && app.config.projects.is_empty());
        assert!(file.is_file());
    }

    #[test]
    fn the_bar_shows_one_panel_at_a_time_and_remembers_it() {
        use crate::sidebar::Panel;
        let mut app = app();
        assert!(app.panels.files && !app.panels.projects);
        // Asking for the other one takes its place.
        let _ = app.update(Message::TogglePanel(Panel::Projects));
        assert!(!app.panels.files && app.panels.projects);
        assert!(app.config.show_projects_panel && !app.config.show_files_panel);
        let _ = app.update(Message::TogglePanel(Panel::Files));
        assert!(app.panels.files && !app.panels.projects);
        // Asking for the one that is open closes it.
        let _ = app.update(Message::TogglePanel(Panel::Files));
        assert!(!app.panels.files && !app.panels.projects);
        // The sidebar was hidden: asking for a panel shows it again.
        app.show_sidebar = false;
        let _ = app.update(Message::TogglePanel(Panel::Projects));
        assert!(app.show_sidebar && app.panels.projects && app.config.show_sidebar);
        // The next run starts as it was left.
        let again = ArdoiseApp::blank(app.config.clone());
        assert!(!again.panels.files && again.panels.projects);
        // An old setting with both open keeps the projects, once there are some.
        let mut old = Config {
            show_files_panel: true,
            show_projects_panel: true,
            ..Config::default()
        };
        let none = ArdoiseApp::blank(old.clone());
        assert!(none.panels.files && !none.panels.projects);
        old.projects = vec![PathBuf::from("/tmp")];
        let some = ArdoiseApp::blank(old);
        assert!(!some.panels.files && some.panels.projects);
    }

    #[test]
    fn a_tab_dragged_elsewhere_keeps_its_new_place() {
        let mut app = app();
        for name in ["a", "b", "c", "d"] {
            let _ = app.update(Message::TabRead(Ok(Loaded {
                path: PathBuf::from(format!("/s/{name}.txt")),
                text: String::new(),
                format: TextFormat::default(),
                stamp: None,
            })));
        }
        let order = |app: &ArdoiseApp| -> Vec<String> {
            app.sidebar
                .files()
                .iter()
                .filter(|p| p.starts_with("/s"))
                .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
                .collect()
        };
        let first = app
            .sidebar
            .files()
            .iter()
            .position(|p| p.starts_with("/s"))
            .unwrap();
        let _ = app.update(Message::TabMoved(first, first + 3));
        assert_eq!(order(&app), ["b", "c", "d", "a"]);
        // It is the order that is saved.
        let saved: Vec<String> = app
            .build_session()
            .files
            .iter()
            .filter(|f| f.path.starts_with("/s"))
            .map(|f| f.path.clone())
            .collect();
        assert_eq!(saved, ["/s/b.txt", "/s/c.txt", "/s/d.txt", "/s/a.txt"]);
    }

    fn open_named(app: &mut ArdoiseApp, path: &Path) {
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.to_path_buf(),
            text: String::new(),
            format: TextFormat::default(),
            stamp: None,
        })));
    }

    #[test]
    fn a_file_of_the_tree_opens_in_the_preview_tab_until_it_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["a.rs", "b.rs", "c.rs"] {
            std::fs::write(dir.path().join(name), "x\n").unwrap();
        }
        let (a, b, c) = (
            dir.path().join("a.rs"),
            dir.path().join("b.rs"),
            dir.path().join("c.rs"),
        );
        let mut app = app();
        let _ = app.update(Message::TreeOpenFile(a.clone()));
        assert_eq!(app.preview_tab.as_ref(), Some(&a));
        open_named(&mut app, &a);
        // Another file of the tree takes its tab, at its place.
        let place = app.sidebar.files().iter().position(|p| *p == a).unwrap();
        app.tree_click = None;
        let _ = app.update(Message::TreeOpenFile(b.clone()));
        open_named(&mut app, &b);
        assert!(
            !app.sidebar.files().contains(&a),
            "the preview was replaced"
        );
        assert_eq!(
            app.sidebar.files().iter().position(|p| *p == b),
            Some(place)
        );
        assert_eq!(app.preview_tab.as_ref(), Some(&b));
        // Edited, it stays: the next file gets a tab of its own.
        let _ = app.update(Message::EditorAction(text_editor::Action::Edit(
            text_editor::Edit::Insert('y'),
        )));
        assert_eq!(app.preview_tab, None);
        app.tree_click = None;
        let _ = app.update(Message::TreeOpenFile(c.clone()));
        open_named(&mut app, &c);
        assert!(app.sidebar.files().contains(&b) && app.sidebar.files().contains(&c));
        // Clicked twice at once, it is kept (the first click is dated now:
        // opening the file may take longer than a double click).
        app.tree_click = Some((c.clone(), std::time::Instant::now()));
        let _ = app.update(Message::TreeOpenFile(c.clone()));
        assert_eq!(app.preview_tab, None);
        // Or from the menu of its tab.
        app.preview_tab = Some(c.clone());
        let _ = app.update(Message::KeepTab);
        assert_eq!(app.preview_tab, None);
        // Or with two clicks on its tab (on another tab, nothing changes).
        app.preview_tab = Some(c.clone());
        let at = app.sidebar.files().iter().position(|p| *p == c).unwrap();
        let other = app.sidebar.files().iter().position(|p| *p == b).unwrap();
        let _ = app.update(Message::TabDoubleClick(other));
        assert_eq!(app.preview_tab.as_ref(), Some(&c));
        let _ = app.update(Message::TabDoubleClick(at));
        assert_eq!(app.preview_tab, None);
    }

    #[test]
    fn tabs_close_with_the_middle_button_and_come_back_with_ctrl_shift_t() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.rs"), dir.path().join("b.rs"));
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        let mut app = app();
        open_named(&mut app, &a);
        open_named(&mut app, &b);
        let at = app.sidebar.files().iter().position(|p| *p == a).unwrap();
        let _ = app.update(Message::CloseTabAt(at));
        assert!(!app.sidebar.files().contains(&a));
        let _ = app.update(Message::ReopenTab);
        assert_eq!(app.pending_open.as_ref(), Some(&a));
        // Nothing left to open again: nothing happens.
        open_named(&mut app, &a);
        app.pending_open = None;
        let _ = app.update(Message::ReopenTab);
        assert_eq!(app.pending_open, None);
    }

    #[test]
    fn ctrl_tab_goes_to_the_tabs_seen_before_while_ctrl_is_held() {
        let mut app = app();
        let tab = |n: &str| PathBuf::from(format!("/s/{n}.txt"));
        for name in ["a", "b", "c"] {
            open_named(&mut app, &tab(name));
        }
        // Seen in the order a, b, c; then b again: b, c, a from the latest.
        let _ = app.update(Message::Sidebar(crate::sidebar::SidebarMessage::OpenFile(
            tab("b"),
        )));
        let ctrl_tab = |app: &mut ArdoiseApp, shift: bool| {
            let _ = app.update(Message::KeyPressed {
                key: "Tab".into(),
                ctrl: true,
                shift,
                alt: false,
            });
        };
        ctrl_tab(&mut app, false);
        assert_eq!(app.current_file, Some(tab("c")), "the one seen before");
        ctrl_tab(&mut app, false);
        assert_eq!(app.current_file, Some(tab("a")), "Ctrl still held: older");
        ctrl_tab(&mut app, true);
        assert_eq!(app.current_file, Some(tab("c")), "and back");
        // Ctrl let go: the tab reached is now the latest.
        let _ = app.update(Message::ModifiersChanged {
            ctrl: false,
            alt: false,
            shift: false,
        });
        ctrl_tab(&mut app, false);
        assert_eq!(app.current_file, Some(tab("b")));
    }

    #[test]
    fn the_list_of_tabs_opens_under_its_button_and_a_pick_closes_it() {
        let mut app = app();
        let file = PathBuf::from("/s/a.txt");
        open_named(&mut app, &file);
        let _ = app.update(Message::OpenTabList(iced::Point::new(900.0, 40.0)));
        assert!(app.tab_list.is_some());
        let _ = app.update(Message::CloseTabList);
        assert!(app.tab_list.is_none());
        let _ = app.update(Message::OpenTabList(iced::Point::new(900.0, 40.0)));
        let _ = app.update(Message::PickTab(file.clone()));
        assert!(app.tab_list.is_none());
        assert_eq!(app.current_file, Some(file));
    }

    #[test]
    fn showing_a_file_in_the_tree_brings_the_projects_up() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "").unwrap();
        let mut app = app();
        let _ = app.update(Message::ProjectPicked(Some(dir.path().to_path_buf())));
        // The files were on show, the sidebar hidden.
        app.panels = crate::sidebar::Panels {
            files: true,
            projects: false,
        };
        app.show_sidebar = false;
        let file = dir.path().join("src/main.rs");
        let _ = app.update(Message::ShowInProjects(file.clone()));
        assert!(app.show_sidebar && app.panels.projects && !app.panels.files);
        assert!(app.config.show_projects_panel && !app.config.show_files_panel);
        assert!(app.projects.rows().iter().any(|r| r.entry.path == file));
        // A file that is in no project: nothing happens.
        let _ = app.update(Message::TogglePanel(crate::sidebar::Panel::Files));
        let _ = app.update(Message::ShowInProjects(PathBuf::from("/elsewhere/x.rs")));
        assert!(app.panels.files && !app.panels.projects);
    }

    /// The Git plugin, installed (its process is not running).
    fn with_git_plugin(app: &mut ArdoiseApp) {
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "ardoise.git", "name": "Git", "version": "1.0.0", "api": 1,
                "command": ["bin/none"],
                "contributes": {"panels": [{"id": "git", "title": "Git", "location": "bottom"}]}}"#,
        )
        .unwrap();
        app.plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
    }

    /// A message of the Git plugin to the editor.
    fn from_git(app: &mut ArdoiseApp, message: ardoise_plugin::protocol::Message) {
        let _ = app.update(Message::Plugin(crate::plugins::PluginMsg::Incoming(
            crate::plugins::host::Incoming::Rpc("ardoise.git".into(), message),
        )));
    }

    /// A diff tab of the Git plugin, of files on the disk.
    fn git_diff_tab(app: &mut ArdoiseApp, tab: &str, title: &str, files: &[&Path]) -> PathBuf {
        let files: Vec<_> = files
            .iter()
            .map(|f| serde_json::json!({"path": f.file_name().unwrap().to_string_lossy(), "patch": "", "mode": "unstaged", "file": f}))
            .collect();
        from_git(
            app,
            ardoise_plugin::protocol::Message::request(
                1,
                ardoise_plugin::protocol::methods::OPEN_DIFF_TAB,
                serde_json::json!({"tab": tab, "title": title, "files": files}),
            ),
        );
        app.current_file.clone().unwrap()
    }

    #[test]
    fn the_preferences_window_is_moved_by_its_title_and_resized_by_its_edges() {
        use crate::preferences::PrefsGrab;
        let mut app = app();
        app.config.window_width = 1200.0;
        app.config.window_height = 800.0;
        app.show_preferences = true;
        let drag = |app: &mut ArdoiseApp, grab: PrefsGrab, by: (f32, f32)| {
            let _ = app.handle(Message::PrefsDragStart(grab));
            // The first move only says where the pointer is.
            let _ = app.handle(Message::PrefsDragMove(iced::Point::new(500.0, 400.0)));
            let _ = app.handle(Message::PrefsDragMove(iced::Point::new(
                500.0 + by.0,
                400.0 + by.1,
            )));
            let _ = app.handle(Message::PrefsDragEnd);
            assert!(app.prefs_drag.is_none());
        };
        // At first: its first size, in the middle.
        assert_eq!(app.prefs_rect(), (150.0, 80.0, 900.0, 640.0));
        // Moved by the title bar.
        drag(&mut app, PrefsGrab::Move, (-100.0, 40.0));
        assert_eq!(app.prefs_rect(), (50.0, 120.0, 900.0, 640.0));
        // It stays inside the window.
        drag(&mut app, PrefsGrab::Move, (-900.0, 900.0));
        assert_eq!(app.prefs_rect(), (0.0, 160.0, 900.0, 640.0));
        drag(&mut app, PrefsGrab::Move, (2000.0, -2000.0));
        assert_eq!(app.prefs_rect(), (300.0, 0.0, 900.0, 640.0));
        // The edges resize one way, the corner both; the top left stays.
        drag(&mut app, PrefsGrab::Bottom, (50.0, -100.0));
        assert_eq!(app.prefs_rect(), (300.0, 0.0, 900.0, 540.0));
        drag(&mut app, PrefsGrab::Right, (-300.0, 70.0));
        assert_eq!(app.prefs_rect(), (300.0, 0.0, 600.0, 540.0));
        drag(&mut app, PrefsGrab::Corner, (100.0, 100.0));
        assert_eq!(app.prefs_rect(), (300.0, 0.0, 700.0, 640.0));
        // Neither smaller than it can be, nor bigger than the window.
        drag(&mut app, PrefsGrab::Corner, (-5000.0, -5000.0));
        assert_eq!(app.prefs_rect(), (300.0, 0.0, 520.0, 360.0));
        drag(&mut app, PrefsGrab::Corner, (5000.0, 5000.0));
        let (x, y, w, h) = app.prefs_rect();
        assert_eq!((w, h), (1200.0, 800.0));
        assert_eq!((x, y), (0.0, 0.0));
        // A window of the app that became small keeps it inside.
        app.config.window_width = 700.0;
        app.config.window_height = 500.0;
        assert_eq!(app.prefs_rect(), (0.0, 0.0, 700.0, 500.0));
        // Moves of the pointer with nothing taken hold of do nothing.
        let before = app.prefs_rect();
        let _ = app.handle(Message::PrefsDragMove(iced::Point::new(9.0, 9.0)));
        assert_eq!(app.prefs_rect(), before);
    }

    #[test]
    fn the_preferences_window_is_in_the_middle_and_fits_a_small_window_at_first() {
        let mut app = app();
        app.config.window_width = 700.0;
        app.config.window_height = 500.0;
        // The margin of 40 px around it, but never under its least size.
        assert_eq!(app.prefs_rect(), (40.0, 40.0, 620.0, 420.0));
        app.config.window_width = 400.0;
        app.config.window_height = 300.0;
        assert_eq!(app.prefs_rect(), (0.0, 0.0, 400.0, 300.0));
    }

    #[test]
    fn deleting_a_file_asks_first() {
        let mut app = app();
        let dir = tempfile::tempdir().unwrap();
        let loose = dir.path().join("loose.txt");
        std::fs::write(&loose, "x").unwrap();
        let _ = app.handle(Message::AskDeleteFile(loose.clone()));
        assert_eq!(
            app.overlay,
            ActiveOverlay::ConfirmFileDelete(loose.clone(), Vec::new())
        );
        let _ = app.handle(Message::DeleteFile(loose.clone()));
        assert!(matches!(app.overlay, ActiveOverlay::None));
        let runtime = tokio::runtime::Runtime::new().unwrap();
        assert_eq!(runtime.block_on(super::delete_file(loose.clone())), Ok(()));
        assert!(!loose.exists());
    }

    #[test]
    fn the_boxes_a_plugin_adds_to_the_deletion_are_shown_and_ticked() {
        use crate::plugins::PluginMsg;
        use ardoise_plugin::protocol::DeleteOption;
        let mut app = app();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tracked.txt");
        std::fs::write(&file, "x").unwrap();
        let option = DeleteOption {
            id: "git_rm".into(),
            label: "Also remove it from Git".into(),
            checked: true,
        };
        let _ = app.handle(Message::Plugin(PluginMsg::DeleteOptions(
            file.clone(),
            vec![("ardoise.git".into(), option.clone())],
        )));
        assert_eq!(
            app.overlay,
            ActiveOverlay::ConfirmFileDelete(file.clone(), vec![("ardoise.git".into(), option)])
        );
        let _ = app.handle(Message::Plugin(PluginMsg::DeleteOptionToggled(0, false)));
        let ActiveOverlay::ConfirmFileDelete(_, options) = &app.overlay else {
            panic!("{:?}", app.overlay);
        };
        assert!(!options[0].1.checked);
        // Out of range: nothing breaks.
        let _ = app.handle(Message::Plugin(PluginMsg::DeleteOptionToggled(5, true)));
        let _ = app.handle(Message::DeleteFile(file.clone()));
        assert!(matches!(app.overlay, ActiveOverlay::None));
    }

    #[test]
    fn ctrl_z_in_a_diff_tab_only_goes_back_over_the_files_of_that_tab() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
        std::fs::write(&a, "a mine\n").unwrap();
        std::fs::write(&b, "b mine\n").unwrap();
        let mut app = app();
        with_git_plugin(&mut app);
        git_diff_tab(&mut app, "changes", "a.txt", &[&a]);
        // b.txt was rolled back last (from elsewhere), a.txt before it.
        app.remember_file(a.clone());
        std::fs::write(&a, "a head\n").unwrap();
        app.remember_file(b.clone());
        std::fs::write(&b, "b head\n").unwrap();
        let _ = app.handle(Message::Undo);
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "a mine\n");
        assert_eq!(
            std::fs::read_to_string(&b).unwrap(),
            "b head\n",
            "not this tab's file"
        );
    }

    #[test]
    fn ctrl_z_in_a_diff_keeps_the_encoding_of_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        let latin1 = b"caf\xe9\n".to_vec();
        std::fs::write(&file, &latin1).unwrap();
        let mut app = app();
        with_git_plugin(&mut app);
        git_diff_tab(&mut app, "commit", "Commit", &[&file]);
        app.remember_file(file.clone());
        std::fs::write(&file, "head\n").unwrap();
        let _ = app.handle(Message::Undo);
        assert_eq!(std::fs::read(&file).unwrap(), latin1);
    }

    #[test]
    fn ctrl_z_in_a_diff_brings_back_what_a_rollback_removed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "mine\n").unwrap();
        let mut app = app();
        with_git_plugin(&mut app);
        git_diff_tab(&mut app, "changes", "Changes", &[&file]);
        // A block put back (the plugin rewrites the file): what it held is
        // kept first.
        let _ = app.handle(Message::DiffTab(DiffNav::Apply(0)));
        std::fs::write(&file, "head\n").unwrap();
        let _ = app.handle(Message::Undo);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "mine\n");
        let _ = app.handle(Message::Redo);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "head\n");
        // Nothing more to go back to.
        let _ = app.handle(Message::Redo);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "head\n");
    }

    #[test]
    fn a_diff_opens_in_an_editor_tab_that_is_not_a_file_and_is_not_saved() {
        let mut app = app();
        with_git_plugin(&mut app);
        let path = git_diff_tab(&mut app, "commit:abc", "abc1234 fix", &[]);
        assert!(is_diff(&path));
        assert!(is_untitled(&path), "no disk access for it");
        assert!(app.diff_tabs.contains_key(&path));
        assert_eq!(display_name(&path), "Δ abc1234 fix");
        // The session forgets it.
        let session = app.build_session();
        assert!(session.files.iter().all(|f| !f.path.starts_with("diff://")));
        assert!(session
            .active_file
            .as_deref()
            .is_none_or(|p| !p.starts_with("diff://")));
        // The same tab id reuses the tab.
        let before = app.sidebar.files().len();
        git_diff_tab(&mut app, "commit:abc", "abc1234 fix", &[]);
        assert_eq!(app.sidebar.files().len(), before);
        // Closing it forgets its state.
        let _ = app.handle(Message::Sidebar(crate::sidebar::SidebarMessage::CloseNow(
            path.clone(),
        )));
        assert!(!app.diff_tabs.contains_key(&path));
    }

    #[test]
    fn git_is_the_git_plugin_and_it_takes_turns_with_the_terminal_at_the_bottom() {
        let mut app = app();
        // Not installed: it is said.
        let _ = app.handle(Message::ToggleGit);
        assert!(app.status_is_error);
        assert_eq!(app.status_message, t!("git.plugin_missing").to_string());
        press(&mut app, "k", true, false, false);
        assert_eq!(app.status_message, t!("git.plugin_missing").to_string());
        // Installed: its panel is a panel of a plugin, at the bottom; the
        // shortcut closes it when it is open.
        with_git_plugin(&mut app);
        let git = ("ardoise.git".to_string(), "git".to_string());
        let _ = app.handle(Message::Plugin(crate::plugins::PluginMsg::TogglePanel(
            git.0.clone(),
            git.1.clone(),
        )));
        assert_eq!(app.plugins.bottom.as_ref(), Some(&git));
        let _ = app.handle(Message::ToggleGit);
        assert!(app.plugins.bottom.is_none());
        // Bookmarks panel: Shift+F11.
        press(&mut app, "F11", false, true, false);
        assert!(app.bookmark_panel.open);
    }

    #[test]
    fn a_conflict_of_the_git_plugin_opens_in_a_tab_that_is_not_a_file_and_is_not_remembered() {
        let mut app = app();
        with_git_plugin(&mut app);
        let open = |app: &mut ArdoiseApp| {
            from_git(
                app,
                ardoise_plugin::protocol::Message::request(
                    2,
                    ardoise_plugin::protocol::methods::OPEN_MERGE,
                    serde_json::json!({"tab": "merge:dir/f.txt", "path": "/tmp/no-such-repo/dir/f.txt",
                        "text": "a\n<<<<<<< HEAD\nmine\n=======\ntheirs\n>>>>>>> t\nb\n"}),
                ),
            );
        };
        open(&mut app);
        let path = app.current_file.clone().unwrap();
        assert!(is_merge(&path) && is_virtual(&path) && is_untitled(&path));
        assert_eq!(display_name(&path), "⇄ f.txt");
        assert!(app.merge_tabs.contains_key(&path));
        let session = app.build_session();
        assert!(session
            .files
            .iter()
            .all(|f| !f.path.starts_with("merge://")));
        // The same file reuses its tab.
        let before = app.sidebar.files().len();
        open(&mut app);
        assert_eq!(app.sidebar.files().len(), before);
        // Settling a conflict in it.
        let _ = app.handle(Message::Merge(crate::diff_view::merge::MergeMsg::All(
            crate::diff::conflicts::Side::Yours,
        )));
        assert_eq!(app.merge_tabs[&path].merge.unresolved(), 0);
        // Not over unsaved changes of the same file.
        let file = PathBuf::from("/tmp/no-such-repo/dir/f.txt");
        app.sidebar.add_file(file.clone());
        app.file_cache.insert(
            file,
            CachedBuffer {
                dirty: true,
                ..CachedBuffer::blank()
            },
        );
        let _ = app.handle(Message::Merge(crate::diff_view::merge::MergeMsg::Apply));
        assert!(app.status_is_error);
        assert_eq!(
            app.status_message,
            t!("git.merge_buffer_unsaved", name = "f.txt").to_string()
        );
        // Closed: forgotten.
        let _ = app.handle(Message::Sidebar(crate::sidebar::SidebarMessage::CloseNow(
            path.clone(),
        )));
        assert!(!app.merge_tabs.contains_key(&path));
    }

    #[test]
    fn a_path_is_shown_with_the_home_folder_as_a_tilde() {
        let home = dirs::home_dir().expect("a home folder");
        assert_eq!(super::display_path(&home.join("work/a.rs")), "~/work/a.rs");
        assert_eq!(super::display_path(Path::new("/etc/hosts")), "/etc/hosts");
    }

    #[test]
    fn closing_the_last_tab_leaves_a_tab_to_type_in() {
        let mut app = app();
        let first = app.current_file.clone().unwrap();
        let _ = app.close_path(first);
        assert!(app.current_file.is_some());
        type_text(&mut app, "hi");
        let session = app.build_session();
        assert!(session
            .files
            .iter()
            .any(|f| f.content.as_deref() == Some("hi\n")));
    }

    #[test]
    fn text_typed_with_no_tab_is_ignored() {
        let mut app = app();
        app.current_file = None;
        type_text(&mut app, "lost");
        assert_eq!(text(&app), "");
    }

    #[test]
    fn a_hidden_editor_is_not_edited() {
        let mut app = app();
        app.editor.language = Some("md".into());
        app.view_mode = ViewMode::Preview;
        assert!(app.editing_blocked());
        let _ = app.handle(Message::InsertTab);
        let _ = app.handle(Message::DeleteLine);
        assert!(!app.is_dirty);
        assert_eq!(text(&app), "");
        // The search card doesn't block the line commands any more.
        app.view_mode = ViewMode::Raw;
        app.show_search = true;
        assert!(!app.editing_blocked());
    }

    #[test]
    fn a_format_result_for_other_text_is_dropped() {
        let mut app = app();
        type_text(&mut app, "a");
        let stale = FormatJob {
            path: app.current_file.clone(),
            source: fingerprint("something else\n"),
        };
        let _ = app.handle(Message::FileFormatted(stale, Ok("zzz\n".into())));
        assert_eq!(text(&app), "a\n");
        assert!(app.status_is_error);

        let fresh = FormatJob {
            path: app.current_file.clone(),
            source: fingerprint(&text(&app)),
        };
        let _ = app.handle(Message::FileFormatted(fresh, Ok("A\n".into())));
        assert_eq!(text(&app), "A\n");
    }

    #[test]
    fn a_format_result_for_another_tab_is_dropped() {
        let mut app = app();
        type_text(&mut app, "a");
        let other = FormatJob {
            path: Some(PathBuf::from("/other.js")),
            source: fingerprint(&text(&app)),
        };
        let _ = app.handle(Message::FileFormatted(other, Ok("zzz\n".into())));
        assert_eq!(text(&app), "a\n");
    }

    #[test]
    fn tab_types_a_real_tab_or_fills_to_the_next_stop() {
        let mut app = app();
        app.config.use_spaces = false;
        let _ = app.handle(Message::InsertTab);
        assert_eq!(text(&app), "\t\n");

        let mut app = self::app();
        app.config.use_spaces = true;
        app.config.tab_width = 4;
        type_text(&mut app, "ab");
        let _ = app.handle(Message::InsertTab);
        assert_eq!(text(&app), "ab  \n");
    }

    #[test]
    fn an_undo_that_reaches_the_saved_text_is_clean() {
        let mut app = app();
        app.saved_fp = Some(fingerprint(""));
        type_text(&mut app, "x");
        assert!(app.is_dirty);
        let _ = app.handle(Message::Undo);
        assert_eq!(text(&app), "");
        assert!(!app.is_dirty);
    }

    #[test]
    fn backspace_at_the_start_leaves_no_undo_step() {
        let mut app = app();
        let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Backspace)));
        let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Delete)));
        assert!(app.undo_stack.is_empty());
        assert!(!app.is_dirty);
    }

    #[test]
    fn the_session_keeps_crlf_and_unsaved_text() {
        let mut app = app();
        app.format.ending = crate::tabs::LineEnding::Crlf;
        type_text(&mut app, "abc");
        let session = app.build_session();

        let mut again = ArdoiseApp::blank(Config::default());
        let _ = again.restore_session(session);
        assert_eq!(text(&again), "abc\n");
        assert!(again.is_dirty);
        assert_eq!(again.format.ending, crate::tabs::LineEnding::Crlf);
    }

    #[test]
    fn a_slow_read_of_a_tab_left_behind_does_not_take_over() {
        let mut app = app();
        let here = app.current_file.clone().unwrap();
        app.pending_open = Some(here.clone());
        let late = Loaded {
            path: PathBuf::from("/late.txt"),
            text: "late\n".into(),
            format: TextFormat::default(),
            stamp: None,
        };
        let _ = app.handle(Message::TabRead(Ok(late)));
        assert_eq!(app.current_file.as_ref(), Some(&here));
    }

    #[test]
    fn external_changes_are_noticed_with_the_size_too() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "one").unwrap();
        let mut app = app();
        app.current_file = Some(file.clone());
        app.disk_stamp = DiskStamp::of(&file);
        assert!(!app.changed_on_disk(&file));
        std::fs::write(&file, "longer content").unwrap();
        assert!(app.changed_on_disk(&file));
    }

    #[test]
    fn reloading_keeps_the_undo_history_and_cursor() {
        let mut app = app();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        app.current_file = Some(file.clone());
        let loaded = |text: &str| Loaded {
            path: file.clone(),
            text: text.into(),
            format: TextFormat::default(),
            stamp: None,
        };
        app.apply_reload(loaded("one\n"));
        app.apply_reload(loaded("two\n"));
        assert_eq!(text(&app), "two\n");
        assert!(!app.is_dirty);
        // The old text is one undo away.
        let _ = app.handle(Message::Undo);
        assert_eq!(text(&app), "one\n");
    }

    /// An app whose scratch folder is a temporary one, with a scratch file
    /// open in the editor.
    fn app_on_scratch() -> (ArdoiseApp, tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app();
        app.scratch.dir = dir.path().join("scratches");
        let _ = app.handle(Message::ScratchName("notes".into()));
        let _ = app.handle(Message::ScratchExt(crate::scratch::Ext(Some("md"))));
        let _ = app.handle(Message::ScratchCreate);
        let path = app.scratch.dir.join("notes.md");
        assert!(path.is_file());
        // The read the app asked for.
        let _ = app.handle(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text: String::new(),
            format: TextFormat::default(),
            stamp: DiskStamp::of(&path),
        })));
        (app, dir, path)
    }

    #[test]
    fn a_scratch_file_made_without_a_name_is_numbered() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app();
        app.scratch.dir = dir.path().join("scratches");
        app.scratch.ext = crate::scratch::Ext(Some("md"));
        for _ in 0..2 {
            let _ = app.handle(Message::ScratchCreate);
        }
        assert!(app.scratch.dir.join("scratch1.md").is_file());
        assert!(app.scratch.dir.join("scratch2.md").is_file());
        assert_eq!(
            crate::scratch::next_name(&app.scratch.dir, "md"),
            "scratch3"
        );
        // Another extension has its own numbering.
        assert_eq!(
            crate::scratch::next_name(&app.scratch.dir, "py"),
            "scratch1"
        );
        assert!(app.scratch.error.is_none());
    }

    #[test]
    fn a_scratch_file_is_made_open_and_kept_out_of_the_file_list() {
        let (app, _dir, path) = app_on_scratch();
        assert_eq!(app.current_file.as_ref(), Some(&path));
        assert!(!app.sidebar.files().contains(&path));
        assert_eq!(app.scratch.files, vec![path]);
        assert!(app.scratch.name.is_empty());
    }

    #[test]
    fn a_scratch_file_is_saved_by_itself_and_never_closes() {
        let (mut app, _dir, path) = app_on_scratch();
        type_text(&mut app, "hello");
        assert!(app.is_dirty);
        let _ = app.handle(Message::AutoSave);
        assert!(!app.is_dirty);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello\n");
        let _ = app.handle(Message::CloseFile);
        assert_eq!(app.current_file.as_ref(), Some(&path));
    }

    #[test]
    fn typing_in_a_scratch_file_is_written_before_leaving_it() {
        let (mut app, _dir, path) = app_on_scratch();
        type_text(&mut app, "kept");
        let other = PathBuf::from("/x/other.txt");
        app.stash_current();
        app.load_buffer(other, CachedBuffer::blank());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "kept\n");
    }

    #[test]
    fn deleting_a_scratch_file_removes_it_and_its_buffer() {
        let (mut app, _dir, path) = app_on_scratch();
        let _ = app.handle(Message::ScratchAskDelete(path.clone()));
        // Nothing goes before the user confirms.
        assert!(matches!(
            app.overlay,
            ActiveOverlay::ConfirmScratchDelete(_)
        ));
        assert!(path.exists());
        let _ = app.handle(Message::ScratchDelete(path.clone()));
        assert!(!path.exists());
        assert!(app.scratch.files.is_empty());
        assert_ne!(app.current_file.as_ref(), Some(&path));
    }

    #[test]
    fn changing_the_folder_moves_the_scratch_files_along() {
        let (mut app, dir, path) = app_on_scratch();
        type_text(&mut app, "moved");
        let target = dir.path().join("elsewhere");
        let _ = app.handle(Message::SetScratchDir(
            target.to_string_lossy().into_owned(),
        ));
        assert!(!path.exists());
        assert_eq!(
            std::fs::read_to_string(target.join("notes.md")).unwrap(),
            "moved\n"
        );
        assert_eq!(app.scratch.dir, target);
        assert_eq!(app.scratch.files, vec![target.join("notes.md")]);
        assert_eq!(app.config.scratch_dir, target.to_string_lossy());
    }

    #[test]
    fn a_double_click_on_a_bookmarked_line_goes_there() {
        use super::bookmark_panel::Target;
        let mut app = app();
        let path = app.current_file.clone().unwrap();
        type_text(&mut app, "one\ntwo\nthree");
        app.bookmarks.toggle(&path, 0);
        let target = Target::Line(path, 0);
        // One click only selects.
        let _ = app.handle(Message::BookmarkClick(target.clone()));
        assert_eq!(app.bookmark_panel.selected.as_ref(), Some(&target));
        assert_eq!(app.editor.content.cursor_position().0, 2);
        let _ = app.handle(Message::BookmarkClick(target));
        assert_eq!(app.editor.content.cursor_position().0, 0);
    }

    #[test]
    fn bookmarks_are_deleted_from_the_panel_and_files_added_from_the_list() {
        use super::bookmark_panel::Target;
        let mut app = app();
        let file = PathBuf::from("/x/a.rs");
        app.bookmarks.toggle(&file, 4);
        let _ = app.handle(Message::ToggleFileBookmark(file.clone()));
        assert!(app.bookmarks.marked_paths().contains(&file));
        let _ = app.handle(Message::BookmarkDelete(Target::Line(file.clone(), 4)));
        assert!(app.bookmarks.lines(&file).is_empty());
        // The file itself stays until its own row is deleted.
        assert!(app.bookmarks.marked_paths().contains(&file));
        let _ = app.handle(Message::BookmarkDelete(Target::File(file.clone())));
        assert!(app.bookmarks.entries().is_empty());
    }

    #[test]
    fn a_background_tab_changed_on_disk_keeps_its_old_text_undoable() {
        let mut app = app();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "base\n").unwrap();
        let loaded = |text: &str| Loaded {
            path: file.clone(),
            text: text.into(),
            format: TextFormat::default(),
            stamp: DiskStamp::of(&file),
        };
        let _ = app.handle(Message::TabRead(Ok(loaded("base\n"))));
        type_text(&mut app, "edit");
        let edited = text(&app);
        // Another tab takes the editor; the file is rolled back meanwhile.
        app.is_dirty = false;
        app.stash_current();
        app.load_buffer(dir.path().join("other.txt"), CachedBuffer::blank());
        let _ = app.handle(Message::TabRead(Ok(loaded("base\n"))));
        assert_eq!(text(&app), "base\n");
        let _ = app.handle(Message::Undo);
        assert_eq!(text(&app), edited);
    }

    #[test]
    fn a_reload_never_overwrites_unsaved_typing() {
        let mut app = app();
        let file = PathBuf::from("/x/a.txt");
        app.current_file = Some(file.clone());
        type_text(&mut app, "mine");
        app.apply_reload(Loaded {
            path: file,
            text: "theirs\n".into(),
            format: TextFormat::default(),
            stamp: None,
        });
        assert_eq!(text(&app), "mine\n");
        assert!(app.external_warned);
    }

    #[test]
    fn enter_indents_after_a_bracket_and_splits_a_pair() {
        let mut first = app();
        type_text(&mut first, "if x {");
        let _ = first.handle(Message::EditorAction(Action::Edit(Edit::Enter)));
        type_text(&mut first, "y");
        // The brace typed was closed on its own: the pair is split.
        assert_eq!(text(&first), "if x {\n    y\n}\n");
        let mut app = app();
        app.editor.content = crate::code_editor::Content::with_text("f({})");
        app.editor.content.select_bytes(0, 3, 0);
        let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Enter)));
        type_text(&mut app, "a");
        assert_eq!(text(&app), "f({\n    a\n})\n");
    }

    #[test]
    fn backspace_takes_the_auto_closed_pair_and_a_closer_dedents() {
        let mut first = app();
        type_text(&mut first, "f(");
        assert_eq!(text(&first), "f()\n");
        let _ = first.handle(Message::EditorAction(Action::Edit(Edit::Backspace)));
        assert_eq!(text(&first), "f\n");
        // A closer typed after indentation alone steps one level back.
        let mut second = app();
        second.editor.content = crate::code_editor::Content::with_text("{\n    ");
        second.editor.content.select_bytes(1, 4, 0);
        let _ = second.handle(Message::EditorAction(Action::Edit(Edit::Insert('}'))));
        assert_eq!(text(&second), "{\n}\n");
    }

    #[test]
    fn a_closed_untitled_tab_with_text_comes_back_with_its_text() {
        let mut app = app();
        type_text(&mut app, "draft");
        let path = app.current_file.clone().unwrap();
        let _ = app.close_path(path);
        let _ = app.handle(Message::ReopenTab);
        assert_eq!(text(&app), "draft\n");
        assert!(app.is_dirty);
    }

    #[test]
    fn undo_brings_the_other_cursors_back() {
        let mut app = app();
        app.editor.content = crate::code_editor::Content::with_text("a\nb\nc\n");
        app.editor.content.select_bytes(0, 0, 0);
        app.editor.content.add_cursor(1, 0);
        app.editor.content.add_cursor(2, 0);
        assert_eq!(app.editor.content.cursor_count(), 3);
        let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Insert('x'))));
        assert_eq!(text(&app), "xa\nxb\nxc\n");
        let _ = app.handle(Message::Undo);
        assert_eq!(text(&app), "a\nb\nc\n");
        assert_eq!(app.editor.content.cursor_count(), 3);
    }

    #[test]
    fn on_a_big_file_the_steps_of_the_same_second_are_one() {
        let mut app = app();
        let big: String = "x".repeat(79) + "\n";
        app.editor.content = crate::code_editor::Content::with_text(&big.repeat(30_000));
        app.editor.content.select_bytes(0, 0, 0);
        let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Enter)));
        let steps = app.undo_stack.len();
        for _ in 0..5 {
            let _ = app.handle(Message::EditorAction(Action::Edit(Edit::Enter)));
        }
        // The first made one; the next ones came within the second.
        assert!(
            app.undo_stack.len() <= steps + 1,
            "{}",
            app.undo_stack.len()
        );
        let _ = app.handle(Message::Undo);
        assert!(app.undo_stack.len() <= steps);
    }

    #[test]
    fn reverting_throws_the_changes_away_and_stays_undoable() {
        let mut app = app();
        // A tab without a file has nothing to go back to.
        let _ = app.handle(Message::RevertFile);
        assert!(app.status_is_error);
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "base\n").unwrap();
        let _ = app.handle(Message::TabRead(Ok(Loaded {
            path: file.clone(),
            text: "base\n".into(),
            format: TextFormat::default(),
            stamp: DiskStamp::of(&file),
        })));
        type_text(&mut app, "edit");
        let edited = text(&app);
        assert!(app.is_dirty);
        let _ = app.handle(Message::RevertFile);
        assert_eq!(text(&app), "base\n");
        assert!(!app.is_dirty);
        let _ = app.handle(Message::Undo);
        assert_eq!(text(&app), edited);
    }

    #[test]
    fn save_as_onto_a_tab_with_unsaved_work_is_refused() {
        let mut app = app();
        let other = PathBuf::from("/x/b.txt");
        app.sidebar.add_file(other.clone());
        app.file_cache.insert(
            other.clone(),
            CachedBuffer {
                dirty: true,
                ..CachedBuffer::blank()
            },
        );
        let origin = app.current_file.clone();
        let _ = app.handle(Message::SaveAsChosen(origin, Ok(other)));
        assert!(app.status_is_error);
    }

    #[test]
    fn save_as_from_a_tab_closed_during_the_dialog_writes_nothing() {
        let mut app = app();
        let closed = PathBuf::from("/x/Untitled-1");
        let _ = app.handle(Message::SaveAsChosen(
            Some(closed),
            Ok(PathBuf::from("/x/notes.txt")),
        ));
        assert!(app.status_is_error);
        assert_eq!(app.status_message, t!("status.save_as_tab_closed"));
    }

    #[test]
    fn save_as_renames_the_tab_and_moves_its_bookmarks() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
        let mut app = app();
        app.sidebar.add_file(a.clone());
        app.current_file = Some(a.clone());
        let _ = app.bookmarks.toggle(&a, 3);
        let _ = app.handle(Message::FileSaved(
            Some(a.clone()),
            Ok((b.clone(), "text\n".into())),
        ));
        assert!(app.sidebar.files().contains(&b));
        assert!(!app.sidebar.files().contains(&a));
        assert_eq!(app.bookmarks.lines(&b), &[3]);
        assert_eq!(app.current_file.as_ref(), Some(&b));
    }

    #[test]
    fn replace_all_keeps_a_trailing_blank_line_and_ignores_no_ops() {
        let mut app = app();
        app.editor.content = crate::code_editor::Content::with_text("a\n\n");
        app.search.query = "zzz".into();
        app.search.replacement = "y".into();
        let _ = app.handle(Message::Search(SearchMessage::ReplaceAll));
        assert_eq!(text(&app), "a\n\n");
        assert!(!app.is_dirty);
        assert!(app.undo_stack.is_empty());
    }

    #[test]
    fn same_named_tabs_get_their_folder() {
        let files = vec![
            PathBuf::from("/p/one/mod.rs"),
            PathBuf::from("/p/two/mod.rs"),
            PathBuf::from("/p/lib.rs"),
        ];
        let labels = unique_labels(&files);
        assert_eq!(labels, ["mod.rs — one", "mod.rs — two", "lib.rs"]);
    }

    #[test]
    fn untitled_tabs_are_named_with_their_number() {
        assert!(display_name(&untitled_path(3)).ends_with(" 3"));
    }

    #[test]
    fn line_commands_keep_the_selection_columns() {
        use crate::app::{remap_col, remap_span};
        assert_eq!(remap_col("a", "    a", 1), 5);
        assert_eq!(remap_col("a", "    a", 0), 0);
        assert_eq!(remap_col("    a", "a", 2), 0);
        let span = Span {
            start: (0, 0),
            end: (1, 1),
        };
        let moved = remap_span("a\nb\n", "    a\n    b\n", span, 0);
        assert_eq!((moved.start, moved.end), ((0, 0), (1, 5)));
    }

    #[test]
    fn a_relative_path_becomes_absolute() {
        let abs = absolute(PathBuf::from("does-not-exist-yet.txt"));
        assert!(abs.is_absolute());
        assert!(abs.ends_with("does-not-exist-yet.txt"));
    }

    #[test]
    fn pasting_lines_moves_the_bookmarks_below() {
        let mut app = app();
        app.editor.content = crate::code_editor::Content::with_text("a\nb\nc\n");
        let path = app.current_file.clone().unwrap();
        let _ = app.bookmarks.toggle(&path, 2);
        let _ = app.handle(Message::ClipboardContent(Some("x\ny\n".into())));
        assert_eq!(app.bookmarks.lines(&path), &[4]);
    }

    #[test]
    fn switching_tabs_keeps_each_cursor_and_text() {
        let mut app = app();
        let first = app.current_file.clone().unwrap();
        type_text(&mut app, "abc");
        let _ = app.handle(Message::NewFile);
        type_text(&mut app, "z");
        let _ = app.activate(first);
        assert_eq!(text(&app), "abc\n");
        assert_eq!(app.editor.content.cursor_position(), (0, 3));
        assert!(app.is_dirty);
    }

    #[test]
    fn a_definition_in_the_same_file_selects_the_name() {
        let mut app = app();
        type_text(&mut app, "x");
        let here = app.current_file.clone().unwrap();
        let hit = crate::definition::Hit {
            path: here,
            line: 0,
            col: 0,
            len: 1,
        };
        app.overlay = ActiveOverlay::References(RefList::searching("x".into()));
        let _ = app.handle(Message::DefinitionFound(
            "x".into(),
            crate::lsp::Outcome::Found(hit),
        ));
        assert_eq!(app.editor.content.selection().as_deref(), Some("x"));
    }

    #[test]
    fn a_definition_in_an_open_tab_switches_to_it() {
        let mut app = app();
        let first = app.current_file.clone().unwrap();
        type_text(&mut app, "call()");
        let _ = app.handle(Message::NewFile);
        let second = app.current_file.clone().unwrap();
        type_text(&mut app, "fn target() {}");
        let _ = app.activate(first);
        let hit = crate::definition::Hit {
            path: second.clone(),
            line: 0,
            col: 3,
            len: 6,
        };
        app.overlay = ActiveOverlay::References(RefList::searching("target".into()));
        let _ = app.handle(Message::DefinitionFound(
            "target".into(),
            crate::lsp::Outcome::Found(hit),
        ));
        assert_eq!(app.current_file.as_ref(), Some(&second));
        assert_eq!(app.editor.content.selection().as_deref(), Some("target"));
        assert!(app.pending_jump.is_none());
    }

    #[test]
    fn back_and_forward_walk_through_the_jumps() {
        let mut app = app();
        type_text(&mut app, "a\nb\nc");
        let _ = app.handle(Message::OpenGotoLine);
        app.goto_line_input = "1".into();
        let _ = app.handle(Message::GotoLineSubmit);
        assert_eq!(app.editor.content.cursor_position().0, 0);
        let _ = app.handle(Message::NavBack);
        assert_eq!(app.editor.content.cursor_position(), (2, 1));
        let _ = app.handle(Message::NavForward);
        assert_eq!(app.editor.content.cursor_position().0, 0);
        // Nothing further forward.
        let _ = app.handle(Message::NavForward);
        assert_eq!(app.editor.content.cursor_position().0, 0);
    }

    #[test]
    fn back_returns_to_the_tab_a_definition_was_followed_from() {
        let mut app = app();
        let first = app.current_file.clone().unwrap();
        type_text(&mut app, "call()");
        let _ = app.handle(Message::NewFile);
        let second = app.current_file.clone().unwrap();
        type_text(&mut app, "fn target() {}");
        let _ = app.activate(first.clone());
        let hit = crate::definition::Hit {
            path: second.clone(),
            line: 0,
            col: 3,
            len: 6,
        };
        app.overlay = ActiveOverlay::References(RefList::searching("target".into()));
        let _ = app.handle(Message::DefinitionFound(
            "target".into(),
            crate::lsp::Outcome::Found(hit),
        ));
        assert_eq!(app.current_file.as_ref(), Some(&second));
        let _ = app.handle(Message::NavBack);
        assert_eq!(app.current_file.as_ref(), Some(&first));
        let _ = app.handle(Message::NavForward);
        assert_eq!(app.current_file.as_ref(), Some(&second));
    }

    #[test]
    fn references_are_listed_and_a_click_goes_to_one() {
        let mut app = app();
        type_text(&mut app, "ab\ncd");
        let here = app.current_file.clone().unwrap();
        let reference = |line, col| crate::lsp::Reference {
            hit: crate::definition::Hit {
                path: here.clone(),
                line,
                col,
                len: 1,
            },
            preview: "x".into(),
        };
        let list = vec![reference(0, 1), reference(1, 0)];
        app.overlay = ActiveOverlay::References(RefList::searching("ab".into()));
        let _ = app.handle(Message::DefinitionFound(
            "ab".into(),
            crate::lsp::Outcome::References(list),
        ));
        assert!(matches!(app.overlay, ActiveOverlay::References(_)));
        let key = |name: &str| Message::KeyPressed {
            key: name.into(),
            ctrl: false,
            shift: false,
            alt: false,
        };
        let _ = app.handle(key("ArrowDown"));
        let ActiveOverlay::References(shown) = &app.overlay else {
            panic!("the list closed")
        };
        assert_eq!((shown.word.as_str(), shown.selected), ("ab", 1));
        let _ = app.handle(key("ArrowDown"));
        let _ = app.handle(key("ArrowUp"));
        let _ = app.handle(key("Return"));
        assert!(matches!(app.overlay, ActiveOverlay::None));
        assert_eq!(app.editor.content.selection().as_deref(), Some("c"));
        let _ = app.handle(Message::NavBack);
        assert_eq!(app.editor.content.cursor_position(), (1, 2));
    }

    #[test]
    fn the_install_offer_gives_way_to_the_next_message_and_to_the_install() {
        let mut app = app();
        let offer = || crate::lsp_install::SERVERS.first();
        app.server_offer = offer();
        app.set_status("other", false);
        assert!(app.server_offer.is_none());
        // The button starts the install and says so in the bar.
        app.server_offer = offer();
        let binary = offer().unwrap().binary;
        let _ = app.handle(Message::InstallServer(binary));
        assert!(app.server_offer.is_none());
        assert!(app.servers.installing.contains(binary));
        assert!(app.status_message.contains(binary));
    }

    #[test]
    fn the_cross_beside_a_file_asks_first_when_it_is_modified() {
        let mut app = app();
        type_text(&mut app, "unsaved work");
        let path = app.current_file.clone().unwrap();
        assert!(app.is_path_dirty(&path));
        let _ = app.handle(Message::Sidebar(crate::sidebar::SidebarMessage::CloseNow(
            path.clone(),
        )));
        assert!(matches!(app.overlay, ActiveOverlay::ConfirmClose(_)));
        // The work is still there until it is confirmed.
        assert!(app.is_dirty);
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "unsaved work\n"
        );
    }

    #[test]
    fn closing_the_other_tabs_keeps_the_active_one_and_asks_once_for_modified_ones() {
        let mut app = app();
        type_text(&mut app, "first");
        let first = app.current_file.clone().unwrap();
        let _ = app.handle(Message::NewFile);
        let second = app.current_file.clone().unwrap();
        let _ = app.handle(Message::NewFile);
        let third = app.current_file.clone().unwrap();
        assert_eq!(app.sidebar.files().len(), 3);
        // The active (third) tab stays; the modified first one needs an answer.
        let _ = app.handle(Message::CloseOtherTabs);
        assert!(matches!(
            app.overlay,
            ActiveOverlay::ConfirmCloseMany(ref paths, 1) if paths == &[first.clone(), second.clone()]
        ));
        assert_eq!(app.sidebar.files().len(), 3);
        let _ = app.handle(Message::ConfirmDiscardMany(vec![first, second]));
        assert!(matches!(app.overlay, ActiveOverlay::None));
        assert_eq!(app.sidebar.files(), std::slice::from_ref(&third));
        assert_eq!(app.current_file, Some(third));
    }

    #[test]
    fn closing_clean_tabs_needs_no_answer_and_all_leaves_a_blank_tab() {
        let mut app = app();
        let _ = app.handle(Message::NewFile);
        let _ = app.handle(Message::NewFile);
        let keep = app.sidebar.files()[0].clone();
        let _ = app.handle(Message::CloseTabs(Some(keep.clone())));
        assert!(matches!(app.overlay, ActiveOverlay::None));
        assert_eq!(app.sidebar.files(), &[keep]);
        let _ = app.handle(Message::CloseTabs(None));
        assert!(matches!(app.overlay, ActiveOverlay::None));
        // The editor is never left without a tab.
        assert_eq!(app.sidebar.files().len(), 1);
        assert!(app.current_file.is_some());
    }

    #[test]
    fn a_click_outside_closes_the_open_menu() {
        let mut app = app();
        let _ = app.handle(Message::ToggleMenu(TopMenu::File));
        assert!(app.open_menu.is_some());
        let _ = app.handle(Message::CloseMenu);
        assert!(app.open_menu.is_none());
    }

    #[test]
    fn editing_the_text_drops_the_blame_column() {
        let mut app = app();
        type_text(&mut app, "one\ntwo");
        let path = app.current_file.clone().unwrap();
        app.blame = Some(BlameState {
            path,
            annotations: crate::code_editor::Annotations {
                lines: std::sync::Arc::new(Vec::new()),
                text: iced::Color::BLACK,
                details: std::sync::Arc::new(Vec::new()),
                link: iced::Color::BLACK,
                card_text: iced::Color::BLACK,
                card_background: iced::Color::BLACK,
                card_border: iced::Color::BLACK,
            },
        });
        assert!(app.blame_shown());
        type_text(&mut app, "x");
        assert!(app.blame.is_none());
    }

    #[test]
    fn a_right_click_in_the_gutter_is_told_from_one_in_the_text() {
        let mut app = app();
        type_text(&mut app, "a\nb");
        let origin = crate::editor::text_origin(&app.config, 2, false);
        let _ = app.handle(Message::EditorRightClick(iced::Point::new(
            origin.x - 20.0,
            origin.y + 5.0,
        )));
        assert!(app.gutter_click);
        let _ = app.handle(Message::EditorRightClick(iced::Point::new(
            origin.x + 30.0,
            origin.y + 5.0,
        )));
        assert!(!app.gutter_click);
    }

    /// A modified file with a place on disk, edited a while ago.
    fn dirty_real_file(app: &mut ArdoiseApp, dir: &Path) -> PathBuf {
        let path = dir.join("a.txt");
        std::fs::write(&path, "one\n").unwrap();
        app.current_file = Some(path.clone());
        app.disk_stamp = DiskStamp::of(&path);
        app.is_dirty = true;
        app.last_edit = std::time::Instant::now() - std::time::Duration::from_secs(5);
        path
    }

    fn app_with_autosave_off() -> ArdoiseApp {
        let config = Config {
            auto_save: false,
            ..Config::default()
        };
        let mut app = ArdoiseApp::blank(config);
        let _ = app.restore_session(crate::session::Session::default());
        app
    }

    #[test]
    fn the_text_revision_moves_with_the_text_only() {
        let mut app = app_with_autosave_off();
        let before = app.text_rev;
        let _ = app.update(Message::OpenGotoLine);
        assert_eq!(app.text_rev, before, "a click is not an edit");
        let _ = app.update(Message::EditorAction(text_editor::Action::Edit(
            text_editor::Edit::Insert('x'),
        )));
        assert!(app.text_rev > before, "typing is");
    }

    #[test]
    fn a_modified_file_is_saved_by_itself_after_a_pause() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app();
        dirty_real_file(&mut app, dir.path());
        let _ = app.handle(Message::AutoSave);
        assert!(app.autosaving > 0, "a save was started");
    }

    #[test]
    fn autosave_waits_for_the_pause_and_respects_the_setting_and_untitled_files() {
        let dir = tempfile::tempdir().unwrap();
        // Still typing.
        let mut typing = app();
        dirty_real_file(&mut typing, dir.path());
        typing.last_edit = std::time::Instant::now();
        let _ = typing.handle(Message::AutoSave);
        assert_eq!(typing.autosaving, 0);
        // Turned off.
        let mut off = app_with_autosave_off();
        dirty_real_file(&mut off, dir.path());
        let _ = off.handle(Message::AutoSave);
        assert_eq!(off.autosaving, 0);
        // Nothing to save: an untitled tab is not a file.
        let mut untitled = app();
        untitled.is_dirty = true;
        untitled.last_edit = std::time::Instant::now() - std::time::Duration::from_secs(5);
        let _ = untitled.handle(Message::AutoSave);
        assert_eq!(untitled.autosaving, 0);
    }

    #[test]
    fn autosave_never_writes_over_a_file_that_changed_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app();
        let path = dirty_real_file(&mut app, dir.path());
        std::fs::write(&path, "changed elsewhere, and longer\n").unwrap();
        let _ = app.handle(Message::AutoSave);
        assert_eq!(app.autosaving, 0);
    }

    #[test]
    fn changed_lines_are_marked_against_what_head_has() {
        let mut app = app();
        type_text(&mut app, "one\ntwo\nthree");
        let path = app.current_file.clone().unwrap();
        app.marks = Some(MarksState {
            path,
            base: Some("one\ntwo\n".into()),
            marks: Default::default(),
            paint: Default::default(),
            rev: (u64::MAX, u64::MAX),
        });
        app.refresh_marks_now();
        let paint = app.change_paint().expect("something changed");
        assert_eq!(paint.added.as_slice(), &[2]);
        // Back to what HEAD has: nothing to paint.
        app.marks.as_mut().unwrap().base = Some("one\ntwo\nthree".into());
        app.marks.as_mut().unwrap().rev = (u64::MAX, u64::MAX);
        app.refresh_marks_now();
        assert!(app.change_paint().is_none());
    }

    fn app_with_a_changed_line() -> ArdoiseApp {
        let mut app = app();
        type_text(&mut app, "one\nTWO\nthree");
        let path = app.current_file.clone().unwrap();
        app.marks = Some(MarksState {
            path,
            base: Some("one\ntwo\nthree\n".into()),
            marks: Default::default(),
            paint: Default::default(),
            rev: (u64::MAX, u64::MAX),
        });
        app.refresh_marks_now();
        app
    }

    #[test]
    fn clicking_a_change_bar_opens_the_button_and_the_button_puts_the_line_back() {
        let mut app = app_with_a_changed_line();
        let _ = app.handle(Message::ChangeBar(1));
        assert_eq!(app.change_paint().unwrap().revert, Some(1));
        let _ = app.handle(Message::RevertChange);
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content).trim_end(),
            "one\ntwo\nthree"
        );
        assert!(app.revert.is_none());
        // And Ctrl+Z brings the change back.
        let _ = app.handle(Message::Undo);
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content).trim_end(),
            "one\nTWO\nthree"
        );
    }

    #[test]
    fn the_button_closes_on_a_second_click_on_the_bar_or_elsewhere_or_after_typing() {
        let mut app = app_with_a_changed_line();
        let _ = app.handle(Message::ChangeBar(1));
        let _ = app.handle(Message::ChangeBar(1));
        assert_eq!(app.change_paint().unwrap().revert, None);
        let _ = app.handle(Message::ChangeBar(1));
        let _ = app.handle(Message::CloseRevert);
        assert_eq!(app.change_paint().unwrap().revert, None);
        // Typing moves the lines under it: it gives up.
        let _ = app.handle(Message::ChangeBar(1));
        type_text(&mut app, "x");
        app.refresh_marks_now();
        assert_eq!(app.change_paint().and_then(|p| p.revert), None);
    }

    #[test]
    fn a_definition_never_used_says_so() {
        let mut app = app();
        app.overlay = ActiveOverlay::References(RefList::searching("lonely".into()));
        let _ = app.handle(Message::DefinitionFound(
            "lonely".into(),
            crate::lsp::Outcome::AtDefinition,
        ));
        assert!(empty_reason(&app).contains("lonely"));
    }

    #[test]
    fn a_missing_definition_is_reported() {
        let mut app = app();
        app.overlay = ActiveOverlay::References(RefList::searching("nothing".into()));
        let _ = app.handle(Message::DefinitionFound(
            "nothing".into(),
            crate::lsp::Outcome::NotFound,
        ));
        assert!(empty_reason(&app).contains("nothing"));
        assert!(!app.status_message.contains("nothing"));
    }

    /// Why the usages popup has nothing to list.
    fn empty_reason(app: &ArdoiseApp) -> String {
        match &app.overlay {
            ActiveOverlay::References(RefList {
                state: RefState::Empty(why),
                ..
            }) => why.clone(),
            other => panic!("not an empty popup: {other:?}"),
        }
    }

    #[test]
    fn the_popup_opens_while_searching_and_a_closed_one_drops_the_answer() {
        let mut app = app();
        type_text(&mut app, "name");
        let _ = app.handle(Message::GotoDefinition);
        // No file yet: nothing starts; with one, the popup shows the wait.
        app.overlay = ActiveOverlay::References(RefList::searching("name".into()));
        let _ = app.handle(Message::CloseOverlay);
        let _ = app.handle(Message::DefinitionFound(
            "name".into(),
            crate::lsp::Outcome::NotFound,
        ));
        assert!(matches!(app.overlay, ActiveOverlay::None));
    }

    #[test]
    fn a_failed_session_write_is_reported_and_retried() {
        let mut app = app();
        app.last_session
            .store(7, std::sync::atomic::Ordering::Relaxed);
        app.session_rev = 3;
        let _ = app.handle(Message::SessionWritten(Err("disk full".into())));
        assert!(app.status_is_error);
        assert_eq!(
            (
                app.last_session.load(std::sync::atomic::Ordering::Relaxed),
                app.session_rev
            ),
            (0, 0)
        );
    }

    #[test]
    fn a_second_window_does_not_write_the_session() {
        let mut app = app();
        app.session_owner = false;
        assert!(app.save_session_now().is_ok());
        assert!(app.session_write_job().is_none());
    }

    fn press(app: &mut ArdoiseApp, key: &str, ctrl: bool, shift: bool, alt: bool) {
        let _ = app.handle(Message::KeyPressed {
            key: key.into(),
            ctrl,
            shift,
            alt,
        });
    }

    #[test]
    fn the_ide_shortcuts_edit_lines() {
        let mut app = app();
        type_text(&mut app, "one\ntwo\nthree");
        // Ctrl+D duplicates the line, Ctrl+Y deletes it.
        press(&mut app, "d", true, false, false);
        assert_eq!(text(&app).trim_end(), "one\ntwo\nthree\nthree");
        press(&mut app, "y", true, false, false);
        assert_eq!(text(&app).trim_end(), "one\ntwo\nthree");
        // Alt+Shift+Up moves it above the one before.
        press(&mut app, "ArrowUp", false, true, true);
        assert_eq!(text(&app).trim_end(), "one\nthree\ntwo");
        // Ctrl+Z goes back, Ctrl+Shift+Z forward.
        press(&mut app, "z", true, false, false);
        assert_eq!(text(&app).trim_end(), "one\ntwo\nthree");
        press(&mut app, "z", true, true, false);
        assert_eq!(text(&app).trim_end(), "one\nthree\ntwo");
    }

    // ---- the Database panel's tables
}

/// Draws the window to a PNG, to look at the design without a screen:
/// `SNAPSHOT=/tmp/look.png cargo test look_tests -- --ignored`.
#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    #[ignore]
    fn the_window_is_drawn_to_a_png() {
        let Ok(out) = std::env::var("SNAPSHOT") else {
            return;
        };
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let config = Config {
            dark_mode: std::env::var("LIGHT").is_err(),
            ..Config::default()
        };
        let mut app = ArdoiseApp::blank(config);
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::ProjectPicked(Some(root.clone())));
        let _ = app.update(Message::ToggleProjectFolder(root.join("src")));
        for name in ["src/theme.rs", "Cargo.toml", "README.md", "src/main.rs"] {
            let path = root.join(name);
            let _ = app.update(Message::TabRead(Ok(Loaded {
                text: std::fs::read_to_string(&path).unwrap(),
                path,
                format: TextFormat::default(),
                stamp: None,
            })));
        }
        let theme = if app.config.dark_mode {
            se_theme::ink_dark()
        } else {
            se_theme::ink_light()
        };
        let settings = iced::Settings {
            default_font: se_theme::UI,
            fonts: se_theme::font_files(),
            ..Default::default()
        };
        let _ = std::fs::remove_file(&out);
        let mut ui = iced_test::Simulator::with_size(settings, (1280.0, 800.0), app.view());
        let shot = ui.snapshot(&theme).unwrap();
        assert!(shot.matches_image(&out).unwrap());
    }
}

#[cfg(test)]
mod text_command_tests {
    use super::*;
    use crate::tabs::{Encoding, LineEnding};
    use text_editor::{Action, Edit};

    fn app_with(text: &str) -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: PathBuf::from("/tmp/text-command-tests/a.txt"),
            text: text.into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        app
    }

    fn buffer(app: &ArdoiseApp) -> String {
        crate::editor::buffer_text(&app.editor.content)
    }

    fn edit(app: &mut ArdoiseApp, edit: Edit) {
        let _ = app.update(Message::EditorAction(Action::Edit(edit)));
    }

    #[test]
    fn backspaces_in_a_row_are_undone_with_one_ctrl_z() {
        let mut app = app_with("hello world");
        let _ = app.update(Message::EditorAction(Action::Move(
            text_editor::Motion::DocumentEnd,
        )));
        let before = app.undo_stack.len();
        for _ in 0..5 {
            edit(&mut app, Edit::Backspace);
        }
        assert_eq!(buffer(&app), "hello \n");
        // One undo point for the five, not a copy of the buffer each.
        assert_eq!(app.undo_stack.len(), before + 1);
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "hello world\n");
    }

    #[test]
    fn a_move_or_another_kind_of_deletion_starts_a_new_undo_step() {
        let mut app = app_with("abcdef");
        let _ = app.update(Message::EditorAction(Action::Move(
            text_editor::Motion::DocumentEnd,
        )));
        edit(&mut app, Edit::Backspace);
        edit(&mut app, Edit::Backspace);
        let _ = app.update(Message::EditorAction(Action::Move(
            text_editor::Motion::DocumentStart,
        )));
        edit(&mut app, Edit::Delete);
        edit(&mut app, Edit::Delete);
        edit(&mut app, Edit::Insert('x'));
        assert_eq!(buffer(&app), "xcd\n");
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "cd\n");
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "abcd\n");
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "abcdef\n");
    }

    #[test]
    fn a_delete_after_backspaces_is_a_step_of_its_own() {
        let mut app = app_with("abcdef");
        let _ = app.update(Message::EditorAction(Action::Move(
            text_editor::Motion::DocumentEnd,
        )));
        for _ in 0..3 {
            let _ = app.update(Message::EditorAction(Action::Move(
                text_editor::Motion::Left,
            )));
        }
        edit(&mut app, Edit::Backspace);
        edit(&mut app, Edit::Delete);
        assert_eq!(buffer(&app), "abef\n");
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "abdef\n");
    }

    #[test]
    fn lines_are_sorted_from_the_menu_of_actions() {
        let mut app = app_with("b\na\nc\n");
        let _ = app.update(Message::LineOp(text_ops::LineOp::SortAscending));
        assert_eq!(buffer(&app), "a\nb\nc\n");
        // One Ctrl+Z puts them back.
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "b\na\nc\n");
    }

    #[test]
    fn a_selection_made_upwards_is_the_one_line_commands_act_on() {
        // Line 10 is empty: guessed from the text, the selection was lines 7
        // to 9.
        let text: String = (0..15)
            .map(|i| {
                if i == 10 {
                    "\n".into()
                } else {
                    format!("l{i}\n")
                }
            })
            .collect();
        let mut app = app_with(&text);
        let content = &mut app.editor.content;
        content.perform(Action::Move(text_editor::Motion::DocumentStart));
        for _ in 0..13 {
            content.perform(Action::Move(text_editor::Motion::Down));
        }
        for _ in 0..3 {
            content.perform(Action::Select(text_editor::Motion::Up));
        }
        assert_eq!(app.lines_to_edit(app.selection_span()), (10, 12));
        let _ = app.update(Message::LineOp(text_ops::LineOp::Reverse));
        let text = buffer(&app);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(&lines[7..14], ["l7", "l8", "l9", "l12", "l11", "", "l13"]);
    }

    #[test]
    fn each_cursor_changes_the_case_of_its_own_selection() {
        // The empty line gives a cursor with nothing selected.
        let mut app = app_with("abcd\n\nefgh\n");
        app.editor.content.column_select((0, 1), (2, 3), 4);
        let _ = app.update(Message::CaseOp(text_ops::CaseOp::Upper));
        assert_eq!(buffer(&app), "aBCd\n\neFGh\n");
        // The selections are still there, each on its own text.
        assert_eq!(
            app.editor.content.selection_all().as_deref(),
            Some("BC\nFG")
        );
        // One Ctrl+Z puts it back.
        let _ = app.update(Message::Undo);
        assert_eq!(buffer(&app), "abcd\n\nefgh\n");
    }

    #[test]
    fn the_word_at_the_cursor_changes_case() {
        let mut app = app_with("fooBar baz\n");
        let _ = app.update(Message::CaseOp(text_ops::CaseOp::Snake));
        assert_eq!(buffer(&app), "foo_bar baz\n");
    }

    #[test]
    fn indentation_is_found_when_a_file_opens_and_converted_on_request() {
        let mut app = app_with("a\n  b\n    c\n  d\ne\n");
        assert_eq!(app.indent(), (true, 2));
        assert_eq!(app.tab_text(), "  ");
        let _ = app.update(Message::ConvertIndent(false));
        assert_eq!(buffer(&app), "a\n\tb\n\t\tc\n\td\ne\n");
        assert_eq!(app.indent(), (false, 2));
        assert_eq!(app.tab_text(), "\t");
        let _ = app.update(Message::SetIndent(true, 8));
        assert_eq!(app.indent(), (true, 8));
    }

    #[test]
    fn the_encoding_and_line_endings_are_chosen_for_the_next_save() {
        let mut app = app_with("café\n");
        assert!(!app.is_dirty);
        let _ = app.update(Message::SetEncoding(Encoding::Latin1));
        assert_eq!(app.format.encoding, Encoding::Latin1);
        assert!(app.is_dirty);
        let _ = app.update(Message::SetLineEnding(LineEnding::Crlf));
        assert_eq!(app.format.ending, LineEnding::Crlf);
        // A character the encoding cannot hold: it stays as it was.
        let mut app = app_with("€\n");
        let _ = app.update(Message::SetEncoding(Encoding::Latin1));
        assert_eq!(app.format.encoding, Encoding::Utf8);
        assert!(app.status_is_error);
    }

    #[test]
    fn a_binary_file_is_not_opened_as_text() {
        let dir = tempfile::tempdir().unwrap();
        // A PNG: its header holds a lone `\r`, which a save would change.
        let png = dir.path().join("a.png");
        std::fs::write(&png, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").unwrap();
        assert_eq!(
            file_io::read_decoded(&png).unwrap_err(),
            t!("status.err_binary").to_string()
        );
        // UTF-16 with its mark holds NULs too, and is text.
        let utf16 = dir.path().join("a.txt");
        std::fs::write(&utf16, [0xff, 0xfe, b'h', 0, b'i', 0]).unwrap();
        assert_eq!(file_io::read_decoded(&utf16).unwrap().0, "hi");
    }

    #[test]
    fn a_file_is_read_again_in_another_encoding() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, [b'c', b'a', b'f', 0xe9, b'\n']).unwrap();
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let (raw, encoding) = file_io::read_decoded(&path).unwrap();
        assert_eq!((raw.as_str(), encoding), ("café\n", Encoding::Windows1252));
        let (text, mut format) = TextFormat::decode(&raw);
        format.encoding = encoding;
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text,
            format,
            stamp: DiskStamp::of(&path),
        })));
        assert_eq!(buffer(&app), "café\n");
        let _ = app.update(Message::ReloadEncoding(Encoding::Latin9));
        assert_eq!(app.format.encoding, Encoding::Latin9);
        // Not UTF-8: it cannot be read that way.
        let _ = app.update(Message::ReloadEncoding(Encoding::Utf8));
        assert_eq!(app.format.encoding, Encoding::Latin9);
        assert!(app.status_is_error);
    }

    #[test]
    fn several_cursors_type_together_and_escape_goes_back_to_one() {
        let mut app = app_with("foo foo\nfoo\n");
        let _ = app.update(Message::AddNextOccurrence(false));
        let _ = app.update(Message::AddNextOccurrence(true));
        assert_eq!(app.editor.content.cursor_count(), 3);
        let _ = app.update(Message::EditorAction(Action::Edit(Edit::Insert('x'))));
        assert_eq!(buffer(&app), "x x\nx\n");
        let _ = app.update(Message::EscapePressed);
        assert_eq!(app.editor.content.cursor_count(), 1);
    }

    #[test]
    fn a_selection_is_surrounded_and_stays_selected() {
        let mut app = app_with("foo bar\n");
        app.config.autocomplete_brackets = false;
        app.editor.content.select_bytes(0, 0, 3);
        let _ = app.update(Message::EditorAction(Action::Edit(Edit::Insert('('))));
        assert_eq!(buffer(&app), "(foo) bar\n");
        assert_eq!(app.editor.content.selection().as_deref(), Some("foo"));
        // Again, with a quote around what is still selected.
        let _ = app.update(Message::EditorAction(Action::Edit(Edit::Insert('"'))));
        assert_eq!(buffer(&app), "(\"foo\") bar\n");
        assert_eq!(app.editor.content.selection().as_deref(), Some("foo"));
    }

    #[test]
    fn the_zoom_changes_the_size_of_the_text_within_its_bounds() {
        let mut app = app_with("a\n");
        let size = app.config.font_size;
        let _ = app.update(Message::Zoom(1));
        assert_eq!(app.config.font_size, size + 1.0);
        let _ = app.update(Message::Zoom(0));
        assert_eq!(app.config.font_size, Config::default().font_size);
        for _ in 0..40 {
            let _ = app.update(Message::Zoom(-1));
        }
        assert_eq!(app.config.font_size, 8.0);
    }
}

#[cfg(test)]
mod large_file_tests {
    use super::*;

    fn file_of(lines: usize) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.log");
        let text: String = (0..lines).map(|i| format!("line {i:05}\n")).collect();
        std::fs::write(&path, text).unwrap();
        (dir, path)
    }

    #[test]
    fn parts_cover_the_file_on_line_boundaries() {
        let (_dir, path) = file_of(1000);
        let size = std::fs::metadata(&path).unwrap().len();
        let (first, start, end) = file_io::read_window(&path, 0, 1000).unwrap();
        assert_eq!(start, 0);
        assert!(first.ends_with('\n') && end <= 1000 && end > 900);
        // The next one begins where the first stopped.
        let (second, start2, end2) = file_io::read_window(&path, end, 1000).unwrap();
        assert_eq!(start2, end);
        assert!(second.starts_with("line "), "{second:?}");
        assert!(end2 > end);
        // The last part ends at the end of the file.
        let (last, _, end) = file_io::read_window(&path, size - 1000, 1000).unwrap();
        assert_eq!(end, size);
        assert!(last.ends_with("line 00999\n"));
        // Going back from a part gives the part just before it.
        let (before, s, e) = file_io::read_window_before(&path, start2, 1000).unwrap();
        assert_eq!(e, start2);
        assert!(s < e && before.ends_with('\n'));
    }

    #[test]
    fn a_window_has_every_line_exactly_once_when_walked_through() {
        let (_dir, path) = file_of(500);
        let size = std::fs::metadata(&path).unwrap().len();
        let mut at = 0;
        let mut all = String::new();
        while at < size {
            let (text, start, end) = file_io::read_window(&path, at, 700).unwrap();
            assert_eq!(start, at, "no gap");
            assert!(end > start, "progress");
            all.push_str(&text);
            at = end;
        }
        assert_eq!(all, std::fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn a_part_of_a_large_file_is_read_in_its_encoding_and_a_binary_one_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        // Windows-1252: « é » is one byte, not `�`.
        let latin = dir.path().join("latin.log");
        std::fs::write(&latin, b"caf\xe9\nth\xe9\n").unwrap();
        let (text, _, end) = file_io::read_window(&latin, 0, 1000).unwrap();
        assert_eq!(text, "café\nthé\n");
        assert_eq!(end, 9, "the bytes of the file, not of the text");
        let bin = dir.path().join("a.bin");
        std::fs::write(&bin, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").unwrap();
        assert_eq!(
            file_io::read_window(&bin, 0, 1000).unwrap_err(),
            t!("status.err_binary").to_string()
        );
    }

    #[test]
    fn utf16_without_a_mark_is_text_too() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        let mut bytes = Vec::new();
        for unit in "hello wörld\nline two\n".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();
        let (text, encoding) = file_io::read_decoded(&path).unwrap();
        assert_eq!(text, "hello wörld\nline two\n");
        assert_eq!(encoding, crate::tabs::Encoding::Utf16Le);
    }

    #[test]
    fn a_large_tab_cannot_be_edited_or_saved() {
        let (_dir, path) = file_of(10);
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text: "line\n".into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        app.large.insert(
            path,
            LargeView {
                start: 0,
                end: 5,
                size: 100 * 1024 * 1024,
            },
        );
        let _ = app.update(Message::EditorAction(text_editor::Action::Edit(
            text_editor::Edit::Insert('x'),
        )));
        assert_eq!(crate::editor::buffer_text(&app.editor.content), "line\n");
        let _ = app.update(Message::SaveFile);
        assert!(app.status_is_error);
        // The next part replaces the text, with nothing to undo.
        let path = app.current_file.clone().unwrap();
        let _ = app.update(Message::LargeLoaded(path, Ok(("more\n".into(), 5, 10))));
        assert_eq!(crate::editor::buffer_text(&app.editor.content), "more\n");
        assert_eq!(app.large_view().unwrap().start, 5);
        assert!(!app.is_dirty);
    }

    #[test]
    fn a_large_file_that_changed_keeps_its_part_and_is_forgotten_when_closed() {
        let (_dir, path) = file_of(10);
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text: "line\n".into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        app.large.insert(
            path.clone(),
            LargeView {
                start: 5,
                end: 10,
                size: 100,
            },
        );
        let _ = app.update(Message::LargeRefreshed(
            path.clone(),
            Ok(("more\n".into(), 5, 12)),
        ));
        let view = app.large_view().unwrap();
        assert_eq!((view.start, view.end), (5, 12));
        assert_eq!(crate::editor::buffer_text(&app.editor.content), "more\n");
        assert!(app.undo_stack.is_empty());
        let _ = app.close_path(path.clone());
        assert!(!app.large.contains_key(&path));
    }

    #[test]
    fn a_large_tab_is_not_reloaded_whole_with_another_encoding() {
        let (_dir, path) = file_of(10);
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text: "line\n".into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        app.large.insert(
            path,
            LargeView {
                start: 0,
                end: 5,
                size: 100 * 1024 * 1024,
            },
        );
        let _ = app.update(Message::ReloadEncoding(crate::tabs::Encoding::Latin9));
        assert!(app.status_is_error);
        assert_eq!(app.status_message, t!("large.read_only"));
        assert_eq!(crate::editor::buffer_text(&app.editor.content), "line\n");
    }
}

#[cfg(test)]
mod language_server_tests {
    use super::*;

    fn app_with(path: &Path, text: &str) -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.to_path_buf(),
            text: text.into(),
            format: TextFormat::default(),
            stamp: DiskStamp::of(path),
        })));
        app
    }

    fn edit(line: usize, col: usize, end_col: usize, text: &str) -> crate::lint::Edit {
        crate::lint::Edit {
            line,
            col,
            end_line: line,
            end_col,
            text: text.into(),
        }
    }

    #[test]
    fn a_rename_changes_the_editor_and_the_other_files_and_is_undone() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.rs"), dir.path().join("b.rs"));
        std::fs::write(&a, "let foo = 1;\nfoo + 1;\n").unwrap();
        std::fs::write(&b, "use foo;\n").unwrap();
        let mut app = app_with(&a, "let foo = 1;\nfoo + 1;\n");
        let source = fingerprint(&crate::editor::buffer_text(&app.editor.content));
        let _ = app.update(Message::Renamed(
            source,
            Ok(vec![
                crate::lsp::FileEdits {
                    path: a.clone(),
                    edits: vec![edit(0, 4, 7, "bar"), edit(1, 0, 3, "bar")],
                },
                crate::lsp::FileEdits {
                    path: b.clone(),
                    edits: vec![edit(0, 4, 7, "bar")],
                },
            ]),
        ));
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "let bar = 1;\nbar + 1;\n"
        );
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "use bar;\n");
        // The editor takes Ctrl+Z; the other files, the action that undoes.
        let _ = app.update(Message::FindUndoReplace);
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "use foo;\n");
        let _ = app.update(Message::Undo);
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "let foo = 1;\nfoo + 1;\n"
        );
    }

    #[test]
    fn a_rename_is_not_applied_to_a_text_that_changed_meanwhile() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.rs");
        std::fs::write(&a, "let foo = 1;\n").unwrap();
        let mut app = app_with(&a, "let foo = 1;\n");
        let _ = app.update(Message::Renamed(
            0,
            Ok(vec![crate::lsp::FileEdits {
                path: a,
                edits: vec![edit(0, 4, 7, "bar")],
            }]),
        ));
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "let foo = 1;\n"
        );
        assert!(app.status_is_error);
    }

    #[test]
    fn a_rename_needs_a_name_and_a_server() {
        let dir = tempfile::tempdir().unwrap();
        let txt = dir.path().join("a.txt");
        std::fs::write(&txt, "foo\n").unwrap();
        let mut app = app_with(&txt, "foo\n");
        let _ = app.update(Message::RenameSymbol);
        assert!(app.status_is_error, "plain text has no server");
        assert_eq!(app.overlay, ActiveOverlay::None);
        let rs = dir.path().join("a.rs");
        std::fs::write(&rs, "  \n").unwrap();
        let mut app = app_with(&rs, "  \n");
        let _ = app.update(Message::RenameSymbol);
        assert!(app.status_is_error, "no name under the cursor");
        let mut app = app_with(&rs, "let foo = 1;\n");
        app.editor.content.select_bytes(0, 5, 0);
        let _ = app.update(Message::RenameSymbol);
        assert_eq!(app.overlay, ActiveOverlay::Rename("foo".into()));
        let _ = app.update(Message::RenameInput("bar".into()));
        assert_eq!(app.overlay, ActiveOverlay::Rename("bar".into()));
        let _ = app.update(Message::CloseOverlay);
        assert_eq!(app.overlay, ActiveOverlay::None);
    }

    #[test]
    fn saving_formats_first_when_asked_and_then_saves() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.rs");
        std::fs::write(&a, "fn  main( ){}\n").unwrap();
        let mut app = app_with(&a, "fn  main( ){}\n");
        app.config.format_on_save = true;
        let _ = app.update(Message::SaveFile);
        assert!(app.is_formatting && app.format_on_save_pending);
        let job = FormatJob {
            path: Some(a.clone()),
            source: fingerprint(&crate::editor::buffer_text(&app.editor.content)),
        };
        let _ = app.update(Message::FileFormatted(job, Ok("fn main() {}\n".into())));
        assert!(!app.format_on_save_pending && !app.is_formatting);
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "fn main() {}\n"
        );
        // The save that followed did not ask for another formatting.
        assert!(!app.skip_format_once);
        // Off: the save goes straight.
        app.config.format_on_save = false;
        let _ = app.update(Message::SaveFile);
        assert!(!app.format_on_save_pending);
    }

    #[test]
    fn a_hover_answer_that_came_late_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.rs");
        std::fs::write(&a, "x\n").unwrap();
        let mut app = app_with(&a, "x\n");
        let at = iced::Point::new(1.0, 2.0);
        let token = app.symbol_token;
        let _ = app.update(Message::SymbolHoverLoaded(
            token,
            at,
            Ok(Some("i32".into())),
        ));
        assert_eq!(app.symbol_card.as_ref().map(|c| c.1.as_str()), Some("i32"));
        // The pointer left, then an old answer comes.
        let _ = app.update(Message::SymbolHover(None));
        assert!(app.symbol_card.is_none());
        let _ = app.update(Message::SymbolHoverLoaded(
            token,
            at,
            Ok(Some("old".into())),
        ));
        assert!(app.symbol_card.is_none());
    }
}

#[cfg(test)]
mod command_line_tests {
    use super::*;

    #[test]
    fn a_file_named_with_a_line_opens_at_that_line_and_column() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "one\ntwo é three\nfour\n").unwrap();
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let loaded = Loaded {
            path: path.clone(),
            text: "one\ntwo é three\nfour\n".into(),
            format: TextFormat::default(),
            stamp: DiskStamp::of(&path),
        };
        let target = crate::ipc::Target {
            path: path.clone(),
            line: Some(2),
            col: Some(6),
        };
        let _ = app.update(Message::TargetOpened(target, Ok(loaded)));
        assert_eq!(app.current_file.as_ref(), Some(&path));
        // Column 6 of "two é three": after "two é" (the é is two bytes).
        assert_eq!(app.editor.content.cursor_position(), (1, 6));
        // A line past the end goes to the last one.
        let target = crate::ipc::Target {
            path: path.clone(),
            line: Some(99),
            col: None,
        };
        let loaded = Loaded {
            path: path.clone(),
            text: "one\ntwo é three\nfour\n".into(),
            format: TextFormat::default(),
            stamp: None,
        };
        let _ = app.update(Message::TargetOpened(target, Ok(loaded)));
        assert_eq!(app.editor.content.cursor_position().1, 0);
    }
}

#[cfg(test)]
mod keyboard_list_tests {
    use super::*;

    fn key(app: &mut ArdoiseApp, name: &str) {
        let _ = app.update(Message::KeyPressed {
            key: name.to_string(),
            ctrl: false,
            shift: false,
            alt: false,
        });
    }

    fn app() -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        app
    }

    #[test]
    fn the_arrows_and_enter_pick_a_row_of_a_small_list() {
        let mut app = app();
        app.spell.choices = vec!["a".into(), "b".into(), "c".into()];
        let _ = app.update(Message::OpenTextOptions(TextOption::Indent));
        app.overlay = ActiveOverlay::SpellLanguages;
        // The window just changed: the first row (automatic) is the one on.
        let _ = app.update(Message::CloseMenu);
        assert_eq!(app.overlay_pick, 0);
        key(&mut app, "ArrowDown");
        key(&mut app, "ArrowDown");
        assert_eq!(app.overlay_pick, 2);
        key(&mut app, "ArrowDown");
        key(&mut app, "ArrowDown");
        assert_eq!(app.overlay_pick, 0, "it wraps around");
        key(&mut app, "ArrowUp");
        assert_eq!(app.overlay_pick, 3);
        key(&mut app, "ArrowUp");
        key(&mut app, "Return");
        assert_eq!(app.overlay, ActiveOverlay::None, "the row was taken");
        assert_eq!(app.config.spell_language, "b");
    }

    #[test]
    fn enter_takes_the_row_of_the_indentation_list() {
        let mut app = app();
        let _ = app.update(Message::OpenTextOptions(TextOption::LineEnding));
        let _ = app.update(Message::CloseMenu);
        key(&mut app, "ArrowDown");
        key(&mut app, "Return");
        assert_eq!(app.overlay, ActiveOverlay::None);
        assert_eq!(app.format.ending, crate::tabs::LineEnding::Crlf);
    }

    #[test]
    fn a_window_without_a_list_leaves_the_keys_alone() {
        let mut app = app();
        assert!(app.overlay_choices().is_empty());
        let _ = app.update(Message::RenameInput("x".into()));
        key(&mut app, "ArrowDown");
        assert_eq!(app.overlay_pick, 0);
    }
}

#[cfg(test)]
mod large_file_write_guards {
    use super::*;

    #[test]
    fn a_large_tab_never_becomes_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.log");
        std::fs::write(&path, "line\n").unwrap();
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text: "line\n".into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        app.large.insert(
            path,
            LargeView {
                start: 0,
                end: 5,
                size: 100 * 1024 * 1024,
            },
        );
        let _ = app.update(Message::SetLineEnding(crate::tabs::LineEnding::Crlf));
        let _ = app.update(Message::SetEncoding(crate::tabs::Encoding::ALL[1]));
        let _ = app.update(Message::Search(SearchMessage::ReplaceAll));
        assert!(!app.is_dirty);
    }
}

#[cfg(test)]
mod terminal_paste {
    #[test]
    fn a_paste_cannot_close_the_bracketed_paste() {
        let safe = super::paste_text_safe("ls\x1b[201~\nrm -rf x\x03\tok\r\n");
        assert_eq!(safe, "ls[201~\nrm -rf x\tok\r\n");
        assert!(!safe.contains('\x1b'));
    }
}
