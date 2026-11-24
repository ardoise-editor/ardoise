//! Spelling in the editor: misspelled words underlined, a window of
//! suggestions for the one under the cursor, a personal dictionary. The
//! dictionaries are a plugin's (`contributes.speller`, the official one is
//! `ardoise.spell`): it is asked `spell/check`, `spell/suggest`,
//! `spell/learn` and `spell/languages`.

use super::*;
use ardoise_plugin::protocol::methods;
use ardoise_plugin::spell::{CheckParams, CheckResult, Language, LearnParams, SuggestParams};
use iced::widget::column;
use std::sync::Arc;

/// Files with more lines than this are not checked as they are typed.
const MAX_LINES: usize = 20_000;
/// Files with more lines than this are checked once the typing pauses:
/// asking at each keystroke would be of no use.
const SYNC_LINES: usize = 300;
/// The pause.
const PAUSE: std::time::Duration = std::time::Duration::from_millis(250);
/// How long the speller may take (the first check reads the dictionary).
const WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// What marks are computed for: the file, its text (revision), the settings.
pub type SpellKey = (Option<PathBuf>, u64, u64);
/// Marks computed, what for, and the language of the dictionary (none:
/// given up, the text having changed again).
pub type SpellChecked = (SpellKey, Option<String>, Vec<(usize, usize, usize)>);

/// What the underlines are numbered from (the problems of the lint are
/// numbered from 0).
pub const MARK_BASE: usize = 1 << 30;

/// Byte ranges of `text` as (line, byte column, end column).
fn line_marks(text: &str, ranges: &[(usize, usize)]) -> Vec<(usize, usize, usize)> {
    let mut starts = vec![0usize];
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    ranges
        .iter()
        .filter(|(from, to)| from < to && *to <= text.len())
        .map(|&(from, to)| {
            let line = starts.partition_point(|s| *s <= from).saturating_sub(1);
            let end = starts.get(line + 1).map_or(to, |next| to.min(next - 1));
            (line, from - starts[line], end - starts[line])
        })
        .collect()
}

/// A misspelled word and what could replace it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub word: String,
    pub line: usize,
    pub col: usize,
    pub list: Vec<String>,
}

#[derive(Default)]
pub struct SpellState {
    /// The language the last check was made in (empty: no dictionary).
    pub language: String,
    /// The misspelled words: (line, byte column, end column).
    pub marks: Vec<(usize, usize, usize)>,
    /// What `marks` were asked for: the file, its text, the settings.
    key: Option<SpellKey>,
    /// The text asked for last: an answer for another one is of no use
    /// (and is not even asked when it is already old).
    wanted: Arc<std::sync::Mutex<Option<SpellKey>>>,
    /// Changes with the language and the words learnt.
    generation: u64,
    /// The languages of the picker.
    pub choices: Vec<String>,
}

impl ArdoiseApp {
    /// The plugin that checks the spelling, started.
    fn speller(&mut self) -> Option<String> {
        let id = self
            .plugins
            .installed
            .iter()
            .find(|p| p.manifest.contributes.speller.is_some())?
            .id()
            .to_string();
        self.plugin_ensure(&id).then_some(id)
    }

    fn has_speller(&self) -> bool {
        self.plugins
            .installed
            .iter()
            .any(|p| p.manifest.contributes.speller.is_some())
    }

    /// Looks again at the words when the text, the file or the settings
    /// changed: at once, or for a big file being typed in, once the typing
    /// pauses. The marks of before stay until the answer.
    pub(super) fn refresh_spelling(&mut self) -> Task<Message> {
        let eligible = self.config.spell_check
            && self.has_speller()
            && self.current_file.is_some()
            && !self.current_file.as_deref().is_some_and(is_virtual)
            && self.large_view().is_none()
            && self.editor.content.line_count() <= MAX_LINES
            && !(self.view_mode == ViewMode::Preview && self.preview_kind().is_some());
        if !eligible {
            if !self.spell.marks.is_empty() {
                self.spell.marks.clear();
            }
            self.spell.key = None;
            return Task::none();
        }
        let key = (
            self.current_file.clone(),
            self.editor.content.revision(),
            self.spell.generation,
        );
        if self.spell.key.as_ref() == Some(&key) {
            return Task::none();
        }
        let typed = self
            .spell
            .key
            .as_ref()
            .is_some_and(|k| k.0 == key.0 && k.2 == key.2);
        // Another file, another language: the old marks mean nothing.
        if !typed {
            self.spell.marks.clear();
        }
        let later = typed && self.editor.content.line_count() > SYNC_LINES;
        self.spell.key = Some(key.clone());
        let Some(id) = self.speller() else {
            return Task::none();
        };
        let text = crate::editor::buffer_text(&self.editor.content);
        let params = CheckParams {
            text: text.clone(),
            syntax: self.editor.language.clone().unwrap_or_default(),
            language: self.config.spell_language.clone(),
            interface: self.config.locale.clone(),
        };
        let wanted = self.spell.wanted.clone();
        if let Ok(mut w) = wanted.lock() {
            *w = Some(key.clone());
        }
        Task::perform(
            async move {
                let still = |w: &Arc<std::sync::Mutex<Option<SpellKey>>>| {
                    w.lock().is_ok_and(|w| w.as_ref() == Some(&key))
                };
                if later {
                    tokio::time::sleep(PAUSE).await;
                    if !still(&wanted) {
                        return None;
                    }
                }
                let answer =
                    crate::plugins::host::request(&id, methods::SPELL_CHECK, &params, WAIT)
                        .await
                        .ok()?;
                let found: CheckResult = serde_json::from_value(answer).ok()?;
                let marks = line_marks(&text, &found.misspelled);
                Some((key, found.language, marks))
            },
            Message::SpellChecked,
        )
    }

    /// The marks of the speller, if still for the text of now.
    pub(super) fn spell_checked(&mut self, done: Option<SpellChecked>) {
        if let Some((_, language, marks)) =
            done.filter(|(key, _, _)| self.spell.key.as_ref() == Some(key))
        {
            self.spell.marks = marks;
            self.spell.language = language.unwrap_or_default();
        }
    }

    /// Asks the suggestions for the misspelled word at the cursor (or just
    /// before it).
    pub(super) fn open_spell_fix(&mut self) -> Task<Message> {
        if !self.has_speller() {
            self.set_status(t!("spell.plugin_missing"), false);
            return Task::none();
        }
        let (line, col) = self.editor.content.cursor_position();
        let found = self
            .spell
            .marks
            .iter()
            .find(|(l, s, e)| *l == line && *s <= col && col <= *e)
            .and_then(|&(_, start, end)| {
                let word = self.editor.content.line(line)?.get(start..end)?.to_string();
                Some(Suggestion {
                    word,
                    line,
                    col: start,
                    list: Vec::new(),
                })
            });
        let Some(found) = found else {
            let key = if self.spell.language.is_empty() {
                t!("spell.no_dictionary")
            } else {
                t!("spell.none")
            };
            self.set_status(key, false);
            return Task::none();
        };
        let Some(id) = self.speller() else {
            return Task::none();
        };
        let params = SuggestParams {
            word: found.word.clone(),
            language: self.spell.language.clone(),
        };
        Task::perform(
            async move {
                let list =
                    crate::plugins::host::request(&id, methods::SPELL_SUGGEST, &params, WAIT)
                        .await
                        .ok()
                        .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
                        .unwrap_or_default();
                Suggestion { list, ..found }
            },
            Message::SpellSuggested,
        )
    }

    /// The suggestions came: the window opens, if the word is still there.
    pub(super) fn spell_suggested(&mut self, found: Suggestion) {
        let still = self
            .editor
            .content
            .line(found.line)
            .and_then(|l| {
                l.get(found.col..found.col + found.word.len())
                    .map(str::to_string)
            })
            .is_some_and(|w| w == found.word);
        if still && matches!(self.overlay, ActiveOverlay::None) {
            self.overlay = ActiveOverlay::Spelling(Box::new(found));
        }
    }

    pub(super) fn spell_replace(&mut self, new: String) -> Task<Message> {
        let ActiveOverlay::Spelling(found) =
            std::mem::replace(&mut self.overlay, ActiveOverlay::None)
        else {
            return Task::none();
        };
        if self.editing_blocked() {
            return Task::none();
        }
        self.push_undo();
        self.undo_new_group = true;
        self.edited(|app| {
            app.editor
                .content
                .select_bytes(found.line, found.col, found.word.len());
            app.editor
                .content
                .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                    std::sync::Arc::new(new),
                )));
        });
        self.is_dirty = true;
        Task::none()
    }

    /// The word is known from now on: for ever (`add`) or until the app
    /// closes. Its marks go at once.
    pub(super) fn spell_learn(&mut self, add: bool) -> Task<Message> {
        let ActiveOverlay::Spelling(found) =
            std::mem::replace(&mut self.overlay, ActiveOverlay::None)
        else {
            return Task::none();
        };
        let lower = found.word.to_lowercase();
        let content = &self.editor.content;
        self.spell.marks.retain(|(line, start, end)| {
            content
                .line(*line)
                .and_then(|l| l.get(*start..*end).map(str::to_lowercase))
                .is_none_or(|w| w != lower)
        });
        let Some(id) = self.speller() else {
            return Task::none();
        };
        let params = LearnParams {
            word: found.word,
            keep: add,
        };
        Task::perform(
            async move {
                crate::plugins::host::request(&id, methods::SPELL_LEARN, &params, WAIT)
                    .await
                    .err()
            },
            Message::SpellLearned,
        )
    }

    /// The speller knows the word: the text is looked at again.
    pub(super) fn spell_learned(&mut self, error: Option<String>) -> Task<Message> {
        if let Some(e) = error {
            self.set_status(format!("{}: {e}", t!("status.error")), true);
        }
        self.spell.generation += 1;
        self.refresh_spelling()
    }

    pub(super) fn open_spell_languages(&mut self) -> Task<Message> {
        let Some(id) = self.speller() else {
            self.set_status(t!("spell.plugin_missing"), true);
            return Task::none();
        };
        Self::ask_spell_languages(id, Message::SpellLanguagesLoaded)
    }

    /// The dictionaries are asked for (the Preferences show them without
    /// opening the picker). Nothing when no plugin checks the spelling.
    pub(super) fn load_spell_choices(&mut self) -> Task<Message> {
        match self.speller() {
            Some(id) => Self::ask_spell_languages(id, Message::SpellChoices),
            None => Task::none(),
        }
    }

    fn ask_spell_languages(id: String, then: fn(Vec<String>) -> Message) -> Task<Message> {
        Task::perform(
            async move {
                crate::plugins::host::request(
                    &id,
                    methods::SPELL_LANGUAGES,
                    serde_json::Value::Null,
                    WAIT,
                )
                .await
                .ok()
                .and_then(|v| serde_json::from_value::<Vec<Language>>(v).ok())
                .unwrap_or_default()
                .into_iter()
                .map(|l| l.code)
                .collect()
            },
            then,
        )
    }

    /// The dictionaries of the speller: the picker opens on them.
    pub(super) fn spell_languages_loaded(&mut self, codes: Vec<String>) {
        if codes.is_empty() {
            self.set_status(t!("spell.no_dictionary"), true);
            return;
        }
        self.spell.choices = codes;
        self.overlay = ActiveOverlay::SpellLanguages;
    }

    pub(super) fn spell_set_language(&mut self, code: String) -> Task<Message> {
        self.overlay = ActiveOverlay::None;
        self.config.spell_language = code;
        self.config.save();
        self.spell.generation += 1;
        let check = self.refresh_spelling();
        let language = if self.config.spell_language.is_empty() {
            t!("spell.language_auto").to_string()
        } else {
            self.config.spell_language.clone()
        };
        self.set_status(t!("spell.language_set", code = language.as_str()), false);
        check
    }

    /// The underlines of the misspelled words, for the editor to draw.
    pub(super) fn spell_marks(&self, dark: bool) -> Vec<crate::code_editor::DiagMark> {
        let color = if dark {
            iced::Color::from_rgb8(0x5f, 0xc7, 0xa5)
        } else {
            iced::Color::from_rgb8(0x1f, 0x9d, 0x76)
        };
        self.spell
            .marks
            .iter()
            .enumerate()
            .map(|(i, (line, col, end))| crate::code_editor::DiagMark {
                id: MARK_BASE + i,
                line: *line,
                col: *col,
                end_line: *line,
                end_col: *end,
                color,
                wavy: true,
                straight: false,
                wash: None,
            })
            .collect()
    }

    pub(super) fn view_spelling(&self, dark: bool, found: &Suggestion) -> Element<'_, Message> {
        let p = se_theme::palette(dark);
        let ghost = move |active: bool| (se_theme::GhostButton { dark, active }).style();
        let mut items: Vec<Element<Message>> = Vec::new();
        if found.list.is_empty() {
            items.push(
                text(t!("spell.no_suggestions").to_string())
                    .size(12.5)
                    .color(p.muted)
                    .into(),
            );
        }
        for (i, suggestion) in found.list.iter().enumerate() {
            items.push(
                button(text(suggestion.clone()).size(13))
                    .padding([6, 12])
                    .width(Length::Fill)
                    .on_press(Message::SpellReplace(suggestion.clone()))
                    .style(ghost(self.overlay_pick == i))
                    .into(),
            );
        }
        let (add_at, ignore_at) = (found.list.len(), found.list.len() + 1);
        let body = column![
            text(found.word.clone())
                .size(14)
                .font(se_theme::MONO)
                .color(se_theme::error_color(dark)),
            column(items).spacing(2),
        ]
        .spacing(12);
        let tool = |label: String, msg: Message, at: usize| -> Element<Message> {
            button(text(label).size(12.5))
                .padding([7, 14])
                .on_press(msg)
                .style(ghost(self.overlay_pick == at))
                .into()
        };
        crate::popin::card(
            dark,
            crate::popin::WIDTH_CARD,
            crate::popin::Head::new("spell-check", t!("spell.title").to_string())
                .sub(found.word.clone()),
            Some(Message::CloseOverlay),
            body.into(),
            Some((
                vec![],
                vec![
                    tool(
                        t!("spell.ignore").to_string(),
                        Message::SpellIgnore,
                        ignore_at,
                    ),
                    tool(t!("spell.add").to_string(), Message::SpellAdd, add_at),
                ],
            )),
        )
    }

    pub(super) fn view_spell_languages(&self, dark: bool) -> Element<'_, Message> {
        let current = &self.spell.language;
        let mut items: Vec<Element<Message>> =
            vec![button(text(t!("spell.language_auto").to_string()).size(13))
                .padding([6, 12])
                .width(Length::Fill)
                .on_press(Message::SpellLanguage(String::new()))
                .style(
                    (se_theme::GhostButton {
                        dark,
                        active: self.config.spell_language.is_empty() || self.overlay_pick == 0,
                    })
                    .style(),
                )
                .into()];
        for (row_index, code) in self.spell.choices.iter().enumerate() {
            items.push(
                button(text(code.clone()).size(13))
                    .padding([6, 12])
                    .width(Length::Fill)
                    .on_press(Message::SpellLanguage(code.clone()))
                    .style(
                        (se_theme::GhostButton {
                            dark,
                            active: (!self.config.spell_language.is_empty() && code == current)
                                || self.overlay_pick == row_index + 1,
                        })
                        .style(),
                    )
                    .into(),
            );
        }
        crate::popin::card(
            dark,
            crate::popin::WIDTH_CARD,
            crate::popin::Head::new("languages", t!("spell.languages").to_string())
                .sub(t!("spell.languages_hint").to_string()),
            Some(Message::CloseOverlay),
            scrollable(column(items).spacing(2))
                .height(Length::Fixed(320.0))
                .into(),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plugin that says it checks the spelling (not started: its answers
    /// are given by hand).
    fn with_speller(app: &mut ArdoiseApp) {
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "test.speller", "name": "Test", "version": "1.0.0", "api": 1,
                "command": ["bin/none"], "contributes": {"speller": {}}}"#,
        )
        .unwrap();
        app.plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
    }

    fn app_with(text: &str, speller: bool) -> ArdoiseApp {
        let mut app = ArdoiseApp::blank(Config::default());
        let _ = app.restore_session(crate::session::Session::default());
        if speller {
            with_speller(&mut app);
        }
        let file = PathBuf::from("/s/a.txt");
        let _ = app.update(Message::TabRead(Ok(Loaded {
            path: file,
            text: text.into(),
            format: TextFormat::default(),
            stamp: None,
        })));
        let _ = app.refresh_spelling();
        app
    }

    /// What the speller would answer for the text of now.
    fn answer(app: &mut ArdoiseApp, ranges: &[(usize, usize)]) {
        let key = app.spell.key.clone().expect("a check was asked");
        let text = crate::editor::buffer_text(&app.editor.content);
        let marks = line_marks(&text, ranges);
        let _ = app.update(Message::SpellChecked(Some((key, Some("xx".into()), marks))));
    }

    #[test]
    fn byte_ranges_become_places_in_their_lines() {
        let text = "hello wrold\nthe nice wrld is\n";
        assert_eq!(
            line_marks(text, &[(6, 11), (21, 25)]),
            vec![(0, 6, 11), (1, 9, 13)]
        );
        // What is out of the text or empty is left out.
        assert!(line_marks(text, &[(3, 3), (40, 50)]).is_empty());
    }

    #[test]
    fn the_answers_of_the_speller_are_shown_if_still_for_the_text() {
        let mut app = app_with("hello wrold\nthe nice wrld is\n", true);
        answer(&mut app, &[(6, 11), (21, 25)]);
        assert_eq!(app.spell.marks, vec![(0, 6, 11), (1, 9, 13)]);
        assert_eq!(app.spell.language, "xx");
        assert_eq!(app.spell_marks(true).len(), 2);
        assert!(app.spell_marks(true)[0].id >= MARK_BASE);
        // Typed in: the marks stay until the next answer, an old one is dropped.
        let old = app.spell.key.clone().unwrap();
        app.editor.content.select_bytes(0, 6, 5);
        let _ = app.update(Message::EditorAction(text_editor::Action::Edit(
            text_editor::Edit::Paste(std::sync::Arc::new("world".into())),
        )));
        let _ = app.refresh_spelling();
        assert_eq!(app.spell.marks.len(), 2);
        let _ = app.update(Message::SpellChecked(Some((old, None, vec![]))));
        assert_eq!(app.spell.marks.len(), 2);
        answer(&mut app, &[(21, 25)]);
        assert_eq!(app.spell.marks, vec![(1, 9, 13)]);
        // Off: nothing is marked.
        app.config.spell_check = false;
        let _ = app.refresh_spelling();
        assert!(app.spell.marks.is_empty());
    }

    #[test]
    fn a_word_is_corrected_added_or_ignored_from_the_window() {
        let mut app = app_with("hello wrold\nwrold\n", true);
        answer(&mut app, &[(6, 11), (12, 17)]);
        app.editor.content.select_bytes(0, 8, 0);
        let _ = app.update(Message::SpellFix);
        // The suggestions come from the speller.
        let _ = app.update(Message::SpellSuggested(Suggestion {
            word: "wrold".into(),
            line: 0,
            col: 6,
            list: vec!["world".into()],
        }));
        let ActiveOverlay::Spelling(found) = app.overlay.clone() else {
            panic!("the window of suggestions");
        };
        assert_eq!(found.list, vec!["world".to_string()]);
        let _ = app.update(Message::SpellReplace("world".into()));
        assert_eq!(
            crate::editor::buffer_text(&app.editor.content),
            "hello world\nwrold\n"
        );
        assert_eq!(app.overlay, ActiveOverlay::None);
        // Ignoring a word takes its marks away at once, everywhere.
        let mut app = app_with("hello wrold\nwrold\n", true);
        answer(&mut app, &[(6, 11), (12, 17)]);
        app.overlay = ActiveOverlay::Spelling(Box::new(Suggestion {
            word: "wrold".into(),
            line: 0,
            col: 6,
            list: vec![],
        }));
        let _ = app.update(Message::SpellIgnore);
        assert!(app.spell.marks.is_empty());
        // Where nothing is wrong, there is no window.
        app.editor.content.select_bytes(0, 1, 0);
        let _ = app.update(Message::SpellFix);
        assert_eq!(app.overlay, ActiveOverlay::None);
    }

    #[test]
    fn without_a_speller_nothing_is_checked_and_the_plugin_is_named() {
        let mut app = app_with("hello wrold\n", false);
        assert!(app.spell.key.is_none() && app.spell.marks.is_empty());
        let _ = app.update(Message::SpellFix);
        assert_eq!(app.status_message, t!("spell.plugin_missing"));
        let _ = app.update(Message::OpenSpellLanguages);
        assert!(app.status_is_error);
    }

    #[test]
    fn a_language_is_chosen_among_the_dictionaries() {
        let mut app = app_with("hello wrold\n", true);
        let _ = app.update(Message::SpellLanguagesLoaded(vec![]));
        assert!(app.status_is_error, "no dictionary");
        let _ = app.update(Message::SpellLanguagesLoaded(vec![
            "en".into(),
            "fr".into(),
        ]));
        assert_eq!(app.overlay, ActiveOverlay::SpellLanguages);
        assert_eq!(app.spell.choices.len(), 2);
        let before = app.spell.key.clone();
        let _ = app.update(Message::SpellLanguage("fr".into()));
        assert_eq!(app.config.spell_language, "fr");
        assert_ne!(app.spell.key, before, "checked again in that language");
    }
}
