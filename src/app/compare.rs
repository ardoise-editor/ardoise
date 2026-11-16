//! Compares two texts side by side, outside Git: the file in front with the
//! clipboard, with its version on disk, or with another file.

use super::*;
use iced::widget::column;

impl SimpleEditApp {
    /// Shows `left` against `right`, each a (name, text).
    pub(super) fn open_comparison(
        &mut self,
        left: (String, String),
        right: (String, String),
    ) -> Task<Message> {
        if left.1 == right.1 {
            self.set_status(t!("compare.same"), false);
            return Task::none();
        }
        let name = format!("{} ↔ {}", left.0, right.0);
        let entry = crate::git_panel::Entry {
            source: crate::git::Source::Tree(String::new()),
            change: crate::git::Change {
                path: name.clone(),
                from: None,
                kind: crate::git::Kind::Modified,
            },
            staged: false,
        };
        self.history_diff = crate::git_panel::Viewer::default();
        self.history_diff.side_by_side = true;
        self.history_diff
            .show(entry, whole_diff(&name, &left.1, &right.1));
        self.overlay = ActiveOverlay::Compare(left.0, right.0);
        self.history_diff.scroll_to_change()
    }

    /// The name and the text of the file in front, as they are now.
    fn compare_current(&self) -> Option<(String, String)> {
        let path = self.current_file.as_ref()?;
        Some((
            file_label(path),
            crate::editor::buffer_text(&self.editor.content),
        ))
    }

    /// The text in front against the clipboard.
    pub(super) fn compare_with_clipboard(&mut self, clipboard: Option<String>) -> Task<Message> {
        let (Some(left), Some(text)) = (self.compare_current(), clipboard) else {
            return Task::none();
        };
        let text = TextFormat::decode(&text).0;
        let text = if text.ends_with('\n') || text.is_empty() {
            text
        } else {
            format!("{text}\n")
        };
        self.open_comparison(left, (t!("compare.clipboard").to_string(), text))
    }

    /// What is on disk against what is in the editor.
    pub(super) fn compare_with_disk(&mut self) -> Task<Message> {
        let Some(path) = self.current_file.clone() else {
            return Task::none();
        };
        // Read off the interface thread, in the encoding of the tab: the
        // detected one could differ and show differences that are not there.
        let encoding = self.format.encoding;
        let read = move || {
            let bytes = file_io::read_bytes(&path)?;
            let raw = encoding
                .decode(&bytes)
                .unwrap_or_else(|| crate::tabs::Encoding::detect(&bytes).0);
            Ok(TextFormat::decode(&raw).0)
        };
        // The tests have no runtime to run a task: it is done on the spot.
        if cfg!(test) {
            return self.compare_disk_read(read());
        }
        Task::perform(
            async move {
                tokio::task::spawn_blocking(read)
                    .await
                    .map_err(|e| e.to_string())?
            },
            Message::CompareDiskRead,
        )
    }

    /// The file as it is on disk has been read: it is set against the editor.
    pub(super) fn compare_disk_read(&mut self, result: Result<String, String>) -> Task<Message> {
        let Some(right) = self.compare_current() else {
            return Task::none();
        };
        match result {
            Ok(disk) => self.open_comparison((t!("compare.on_disk").to_string(), disk), right),
            Err(e) => {
                self.set_status(format!("{}: {e}", t!("status.error")), true);
                Task::none()
            }
        }
    }

    /// Asks for a file to compare the text in front with.
    pub(super) fn compare_pick_file(&mut self) -> Task<Message> {
        Task::perform(
            async {
                let handle = rfd::AsyncFileDialog::new()
                    .set_title(t!("compare.pick").to_string())
                    .pick_file()
                    .await
                    .ok_or_else(|| CANCELLED.to_string())?;
                let path = handle.path().to_path_buf();
                let (raw, _) = file_io::read_decoded(&path)?;
                Ok((path, TextFormat::decode(&raw).0))
            },
            Message::CompareFilePicked,
        )
    }

    pub(super) fn compare_with_file(
        &mut self,
        result: Result<(PathBuf, String), String>,
    ) -> Task<Message> {
        match result {
            Ok((path, text)) => {
                let Some(left) = self.compare_current() else {
                    return Task::none();
                };
                self.open_comparison(left, (file_label(&path), text))
            }
            Err(e) if e == CANCELLED => Task::none(),
            Err(e) => {
                self.set_status(format!("{}: {e}", t!("status.error")), true);
                Task::none()
            }
        }
    }

    pub(super) fn view_compare(&self, dark: bool, left: &str, right: &str) -> Element<'_, Message> {
        let p = se_theme::palette(dark);
        let header = row![
            text(left.to_string()).size(13).font(se_theme::UI_SEMIBOLD),
            text("↔").size(13).color(p.muted),
            text(right.to_string()).size(13).font(se_theme::UI_SEMIBOLD),
            Space::new().width(Length::Fill),
            button(text("×").size(17))
                .padding([4, 8])
                .on_press(Message::CloseOverlay)
                .style(
                    (se_theme::GhostButton {
                        dark,
                        active: false,
                    })
                    .style()
                ),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        container(
            column![
                container(header).padding([8, 14]),
                crate::git_panel::diff_view::view(
                    &self.history_diff,
                    &crate::git_panel::diff_view::Nav::compare(),
                    dark,
                    String::new(),
                ),
            ]
            .spacing(0),
        )
        .width(1100)
        .height(640)
        .style(se_theme::palette_card(dark))
        .into()
    }
}

/// it, for the two files side by side.
pub(super) fn whole_diff(name: &str, old: &str, new: &str) -> crate::git::FileDiff {
    let body = similar::TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(usize::MAX / 4)
        .header(&format!("a/{name}"), &format!("b/{name}"))
        .to_string();
    let text = format!("diff --git a/{name} b/{name}\n{body}");
    crate::git::diff::parse_diff(&text)
        .into_iter()
        .next()
        .unwrap_or_else(|| crate::git::FileDiff {
            path: name.to_string(),
            ..Default::default()
        })
}

impl SimpleEditApp {
    /// The arrows and picks of the diff of a comparison.
    pub(super) fn compare_diff_nav(&mut self, nav: DiffNav) -> Task<Message> {
        let viewer = &mut self.history_diff;
        let nowhere = Path::new("");
        match nav {
            DiffNav::NextChange => viewer.next_change(nowhere),
            DiffNav::PrevChange => viewer.prev_change(nowhere),
            DiffNav::Pick(pick) => {
                viewer.pick(pick);
                Task::none()
            }
            DiffNav::Scrolled(top, height) => {
                viewer.view_top.set(top as f32);
                viewer.view_height.set(height as f32);
                Task::none()
            }
            DiffNav::CopyPicked => match viewer.selected_text() {
                Some(text) => iced::clipboard::write(text),
                None => Task::none(),
            },
            DiffNav::PickAll => {
                viewer.select_all();
                Task::none()
            }
            DiffNav::ToggleSide => {
                viewer.side_by_side = !viewer.side_by_side;
                viewer.selection = None;
                viewer.scroll_to_change()
            }
            _ => Task::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with(text: &str) -> SimpleEditApp {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, text).unwrap();
        let mut app = SimpleEditApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: path.clone(),
            text: text.into(),
            format: TextFormat::default(),
            stamp: DiskStamp::of(&path),
        })));
        // The folder lives as long as the test needs the file.
        std::mem::forget(dir);
        app
    }

    #[test]
    fn the_text_in_front_is_compared_with_the_clipboard() {
        let mut app = app_with("one\ntwo\nthree\n");
        let _ = app.update(Message::CompareClipboard(Some("one\n2\nthree".into())));
        assert_eq!(
            app.overlay,
            ActiveOverlay::Compare("a.txt".into(), t!("compare.clipboard").to_string())
        );
        assert_eq!(app.history_diff.changes(), 1);
        // The same text: nothing to show, and it is said.
        let mut app = app_with("one\n");
        let _ = app.update(Message::CompareClipboard(Some("one\n".into())));
        assert_eq!(app.overlay, ActiveOverlay::None);
        assert!(!app.status_is_error);
    }

    #[test]
    fn what_was_typed_is_compared_with_the_file_on_disk() {
        let mut app = app_with("one\ntwo\n");
        let _ = app.update(Message::CompareDisk);
        assert_eq!(app.overlay, ActiveOverlay::None, "nothing changed yet");
        app.editor.content.select_bytes(0, 3, 0);
        let _ = app.update(Message::EditorAction(text_editor::Action::Edit(
            text_editor::Edit::Insert('!'),
        )));
        let _ = app.update(Message::CompareDisk);
        assert!(matches!(app.overlay, ActiveOverlay::Compare(..)));
        assert_eq!(app.history_diff.changes(), 1);
        let _ = app.update(Message::CloseOverlay);
        assert_eq!(app.overlay, ActiveOverlay::None);
    }

    #[test]
    fn another_file_is_compared_and_a_cancelled_dialog_does_nothing() {
        let mut app = app_with("a\nb\n");
        let _ = app.update(Message::CompareFilePicked(Err(CANCELLED.to_string())));
        assert_eq!(app.overlay, ActiveOverlay::None);
        assert!(!app.status_is_error);
        let _ = app.update(Message::CompareFilePicked(Ok((
            PathBuf::from("/x/other.txt"),
            "a\nc\n".into(),
        ))));
        assert_eq!(
            app.overlay,
            ActiveOverlay::Compare("a.txt".into(), "other.txt".into())
        );
    }

    #[test]
    fn the_whole_diff_has_every_line_and_its_blocks() {
        let old = "a\nb\nc\nd\ne\nf\ng\nh\n";
        let new = "a\nB\nc\nd\ne\nf\ng\nh\n";
        let diff = whole_diff("x.rs", old, new);
        let lines: usize = diff.hunks.iter().map(|h| h.lines.len()).sum();
        assert_eq!(lines, 9, "the eight lines, one of them twice");
        let blocks = diff.blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].at, 1);
    }
}
