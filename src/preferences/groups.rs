//! The settings of the Preferences as data: each one is a row with its
//! group, its label and its control. The pages draw them by group, the search
//! looks at their labels, and a group can be put back to its defaults.

use iced::widget::{
    button, column, container, pick_list, row, slider, text, text_editor, text_input, Space,
};
use iced::{Alignment, Element, Length};

use super::{
    card_of_rows, plain, setting_row, switch, PreferencesMessage, PreferencesState, PrefsPage,
};
use crate::app::Message;
use crate::config::Config;
use crate::theme;

/// A block of settings that belong together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Display,
    Indentation,
    Typing,
    Completion,
    Files,
    Search,
    Panels,
    Projects,
    Scratch,
    Snippets,
    Trust,
    History,
    Language,
    Appearance,
    /// On the page of the Git plugin.
    Git,
    /// On the page of the spelling plugin.
    Spelling,
}

impl Group {
    pub fn title(self) -> String {
        match self {
            Group::Display => t!("prefs.group_display"),
            Group::Indentation => t!("prefs.group_indentation"),
            Group::Typing => t!("prefs.group_typing"),
            Group::Completion => t!("prefs.group_completion"),
            Group::Files => t!("prefs.group_files"),
            Group::Search => t!("prefs.group_search"),
            Group::Panels => t!("prefs.group_panels"),
            Group::Projects => t!("prefs.group_projects"),
            Group::Scratch => t!("prefs.group_scratch"),
            Group::Snippets => t!("prefs.group_snippets"),
            Group::Trust => t!("prefs.group_trust"),
            Group::History => t!("prefs.group_history"),
            Group::Language => t!("prefs.group_language"),
            Group::Appearance => t!("prefs.group_appearance"),
            Group::Git => t!("git.title"),
            Group::Spelling => t!("prefs.group_spelling"),
        }
        .to_string()
    }

    /// The page the group is on (`None`: the page of a plugin).
    pub fn page(self) -> Option<PrefsPage> {
        Some(match self {
            Group::Completion => PrefsPage::Servers,
            Group::Appearance => PrefsPage::Appearance,
            Group::Snippets => PrefsPage::Snippets,
            Group::Git | Group::Spelling => return None,
            _ => PrefsPage::Editor,
        })
    }

    /// Whether the group has a Reset button: the ones made of switches and
    /// choices, not the lists the user wrote.
    pub fn resettable(self) -> bool {
        matches!(
            self,
            Group::Display
                | Group::Indentation
                | Group::Typing
                | Group::Completion
                | Group::Files
                | Group::Search
        )
    }

    /// Where the group is, as the results of a search say it.
    fn place(self) -> String {
        match self.page() {
            Some(page) => format!("{} › {}", page.label(), self.title()),
            None => self.title(),
        }
    }
}

/// One setting: its place, its label (what the search looks at) and its row.
pub struct Setting<'a> {
    pub group: Group,
    pub label: String,
    pub element: Element<'a, Message>,
}

/// What the rows need to know besides the settings.
pub struct Context<'a> {
    pub config: &'a Config,
    pub dark: bool,
    pub plugins: &'a crate::plugins::Plugins,
    /// The languages of the dictionaries found.
    pub spell_choices: &'a [String],
}

fn msg(m: PreferencesMessage) -> Message {
    Message::Preferences(m)
}

fn small_button(dark: bool, label: String, on_press: Option<Message>) -> Element<'static, Message> {
    button(text(label).size(12))
        .padding([6, 12])
        .on_press_maybe(on_press)
        .style((theme::Outline { dark }).style())
        .into()
}

/// Every setting of the editor, in the order the pages show them.
pub fn all<'a>(state: &'a PreferencesState, ctx: &Context<'_>) -> Vec<Setting<'a>> {
    let (config, dark) = (ctx.config, ctx.dark);
    let mut out: Vec<Setting> = Vec::new();
    let mut add = |group: Group, label: String, element: Element<'a, Message>| {
        out.push(Setting {
            group,
            label,
            element,
        });
    };
    let toggle = |label: String, on: bool, message: fn(bool) -> PreferencesMessage| {
        (
            label.clone(),
            setting_row(label, switch(dark, on, msg(message(!on)))),
        )
    };
    // A set of segments (a width, a column).
    let segments = |current: usize, values: &[usize], message: fn(usize) -> PreferencesMessage| {
        let buttons: Vec<Element<'a, Message>> = values
            .iter()
            .map(|&value| {
                button(text(value.to_string()).size(12).font(theme::MONO))
                    .padding([3, 10])
                    .on_press(msg(message(value)))
                    .style(
                        (theme::Segment {
                            dark,
                            active: current == value,
                            radius: theme::R_SM,
                        })
                        .style(),
                    )
                    .into()
            })
            .collect();
        container(row(buttons).spacing(2))
            .padding(2)
            .style(theme::field(dark, false, theme::R_ROW))
    };

    // ── Display ─────────────────────────────────────────────────────────
    let font_size = row![
        slider(8.0..=32.0, config.font_size, |v| {
            msg(PreferencesMessage::FontSizeChanged(v.round()))
        })
        .step(1.0_f32)
        .width(140)
        .style((theme::Slider { dark }).style()),
        text(format!("{:.0} px", config.font_size))
            .size(13)
            .font(theme::MONO)
            .width(48)
            .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let label = t!("prefs.font_size").to_string();
    add(
        Group::Display,
        label.clone(),
        setting_row(label, font_size.into()),
    );
    // The fixed-width fonts installed, to choose from (the bundled one first).
    let default = t!("prefs.font_default").to_string();
    let mut fonts = vec![default.clone()];
    fonts.extend(theme::installed_mono_fonts().iter().cloned());
    let chosen = if config.font_family.is_empty() {
        default.clone()
    } else {
        config.font_family.clone()
    };
    if !fonts.contains(&chosen) {
        fonts.push(chosen.clone());
    }
    let family = pick_list(fonts, Some(chosen), move |picked| {
        msg(PreferencesMessage::FontFamilySet(if picked == default {
            String::new()
        } else {
            picked
        }))
    })
    .padding([6, 12])
    .width(220)
    .text_size(13)
    .style(theme::Select { dark }.style())
    .menu_style(theme::Select { dark }.menu_style());
    let label = t!("prefs.font_family").to_string();
    add(
        Group::Display,
        label.clone(),
        setting_row(label, family.into()),
    );
    for (label, on, message) in [
        (
            t!("prefs.show_line_numbers").to_string(),
            config.show_line_numbers,
            PreferencesMessage::ShowLineNumbersToggled as fn(bool) -> PreferencesMessage,
        ),
        (
            t!("prefs.highlight_current_line").to_string(),
            config.highlight_current_line,
            PreferencesMessage::HighlightCurrentLineToggled,
        ),
        (
            t!("prefs.show_minimap").to_string(),
            config.show_minimap,
            PreferencesMessage::ShowMinimapToggled,
        ),
        (
            t!("prefs.show_page_guide").to_string(),
            config.show_page_guide,
            PreferencesMessage::ShowPageGuideToggled,
        ),
    ] {
        let (label, element) = toggle(label, on, message);
        add(Group::Display, label, element);
    }
    let label = t!("prefs.page_guide_column").to_string();
    add(
        Group::Display,
        label.clone(),
        setting_row(
            label,
            segments(
                config.page_guide_column,
                &[72, 80, 100, 120],
                PreferencesMessage::PageGuideColumnChanged,
            )
            .into(),
        ),
    );
    for (label, on, message) in [
        (
            t!("prefs.show_indent_guides").to_string(),
            config.show_indent_guides,
            PreferencesMessage::ShowIndentGuidesToggled as fn(bool) -> PreferencesMessage,
        ),
        (
            t!("prefs.show_whitespace").to_string(),
            config.show_whitespace,
            PreferencesMessage::ShowWhitespaceToggled,
        ),
        (
            t!("prefs.word_wrap").to_string(),
            config.word_wrap,
            PreferencesMessage::WordWrapToggled,
        ),
    ] {
        let (label, element) = toggle(label, on, message);
        add(Group::Display, label, element);
    }

    // ── Indentation ─────────────────────────────────────────────────────
    let label = t!("prefs.tab_width").to_string();
    add(
        Group::Indentation,
        label.clone(),
        setting_row(
            label,
            segments(
                config.tab_width,
                &[2, 4, 8],
                PreferencesMessage::TabWidthChanged,
            )
            .into(),
        ),
    );
    for (label, on, message) in [
        (
            t!("prefs.use_spaces").to_string(),
            config.use_spaces,
            PreferencesMessage::UseSpacesToggled as fn(bool) -> PreferencesMessage,
        ),
        (
            t!("prefs.auto_indent").to_string(),
            config.auto_indent,
            PreferencesMessage::AutoIndentToggled,
        ),
        (
            t!("prefs.detect_indent").to_string(),
            config.detect_indent,
            PreferencesMessage::DetectIndentToggled,
        ),
    ] {
        let (label, element) = toggle(label, on, message);
        add(Group::Indentation, label, element);
    }

    // ── Typing ──────────────────────────────────────────────────────────
    for (label, on, message) in [
        (
            t!("prefs.autocomplete_brackets").to_string(),
            config.autocomplete_brackets,
            PreferencesMessage::AutocompleteBracketsToggled as fn(bool) -> PreferencesMessage,
        ),
        (
            t!("prefs.autocomplete_quotes").to_string(),
            config.autocomplete_quotes,
            PreferencesMessage::AutocompleteQuotesToggled,
        ),
        (
            t!("prefs.match_brackets").to_string(),
            config.match_brackets,
            PreferencesMessage::MatchBracketsToggled,
        ),
        (
            t!("prefs.highlight_word").to_string(),
            config.highlight_word,
            PreferencesMessage::HighlightWordToggled,
        ),
    ] {
        let (label, element) = toggle(label, on, message);
        add(Group::Typing, label, element);
    }

    // ── Completion ──────────────────────────────────────────────────────
    for (label, on, message) in [
        (
            t!("prefs.completion").to_string(),
            config.completion_enabled,
            PreferencesMessage::CompletionToggled as fn(bool) -> PreferencesMessage,
        ),
        (
            t!("prefs.completion_auto").to_string(),
            config.completion_auto,
            PreferencesMessage::CompletionAutoToggled,
        ),
    ] {
        let (label, element) = toggle(label, on, message);
        add(Group::Completion, label, element);
    }
    // What each language's server proposes.
    for (key, label) in [
        (
            "typescript.auto_import",
            t!("prefs.completion_ts_imports").to_string(),
        ),
        (
            "rust.auto_import",
            t!("prefs.completion_rust_imports").to_string(),
        ),
        (
            "rust.postfix",
            t!("prefs.completion_rust_postfix").to_string(),
        ),
        (
            "rust.call_arguments",
            t!("prefs.completion_rust_arguments").to_string(),
        ),
        (
            "python.auto_import",
            t!("prefs.completion_python_imports").to_string(),
        ),
    ] {
        let on = config.completion_options.get(key).copied().unwrap_or(true);
        add(
            Group::Completion,
            label.clone(),
            setting_row(
                label,
                switch(
                    dark,
                    on,
                    msg(PreferencesMessage::CompletionOption(key.to_string(), !on)),
                ),
            ),
        );
    }

    // ── Files ───────────────────────────────────────────────────────────
    for (label, on, message) in [
        (
            t!("prefs.auto_save").to_string(),
            config.auto_save,
            PreferencesMessage::AutoSaveToggled as fn(bool) -> PreferencesMessage,
        ),
        (
            t!("prefs.format_on_save").to_string(),
            config.format_on_save,
            PreferencesMessage::FormatOnSaveToggled,
        ),
        (
            t!("prefs.trim_trailing_whitespace").to_string(),
            config.trim_trailing_whitespace,
            PreferencesMessage::TrimTrailingToggled,
        ),
        (
            t!("prefs.final_newline_on_save").to_string(),
            config.final_newline_on_save,
            PreferencesMessage::FinalNewlineToggled,
        ),
    ] {
        let (label, element) = toggle(label, on, message);
        add(Group::Files, label, element);
    }
    let encodings: Vec<String> = crate::tabs::Encoding::ALL
        .iter()
        .map(|e| e.label().to_string())
        .collect();
    let chosen = crate::tabs::Encoding::ALL
        .iter()
        .find(|e| e.label() == config.default_encoding)
        .copied()
        .unwrap_or_default()
        .label()
        .to_string();
    let select = pick_list(encodings, Some(chosen), |label| {
        msg(PreferencesMessage::DefaultEncodingChanged(label))
    })
    .padding([6, 12])
    .width(150)
    .text_size(13)
    .style(theme::Select { dark }.style())
    .menu_style(theme::Select { dark }.menu_style());
    let label = t!("prefs.default_encoding").to_string();
    add(
        Group::Files,
        label.clone(),
        setting_row(label, select.into()),
    );
    let crlf = config.default_line_ending.eq_ignore_ascii_case("crlf");
    let ending = |name: &str, on: bool| {
        button(text(name.to_string()).size(12).font(theme::MONO))
            .padding([3, 12])
            .on_press(msg(PreferencesMessage::DefaultLineEndingChanged(
                name.to_string(),
            )))
            .style(
                (theme::Segment {
                    dark,
                    active: on,
                    radius: theme::R_SM,
                })
                .style(),
            )
    };
    let label = t!("prefs.default_line_ending").to_string();
    add(
        Group::Files,
        label.clone(),
        setting_row(
            label,
            container(row![ending("LF", !crlf), ending("CRLF", crlf)].spacing(2))
                .padding(2)
                .style(theme::field(dark, false, theme::R_ROW))
                .into(),
        ),
    );

    // ── Search everywhere ───────────────────────────────────────────────
    let (label, element) = toggle(
        t!("prefs.everywhere_text").to_string(),
        config.everywhere_text,
        PreferencesMessage::EverywhereTextToggled,
    );
    add(Group::Search, label, element);
    // The abbreviations the user gave to actions.
    let labels = crate::palette::entries(
        config,
        true,
        true,
        crate::plugins::Tools {
            git: true,
            terminal: true,
        },
        &crate::menus::EditorState::default(),
    );
    let mut any = false;
    for (command, list) in &config.action_abbreviations {
        let name = labels
            .iter()
            .find(|e| e.command.id() == *command)
            .map_or_else(|| command.clone(), |e| e.label.clone());
        for abbreviation in list {
            any = true;
            let label = format!("{name} · {abbreviation}");
            add(
                Group::Search,
                label.clone(),
                setting_row(
                    format!("{} {}", t!("prefs.abbreviation"), label),
                    small_button(
                        dark,
                        t!("prefs.remove").to_string(),
                        Some(msg(PreferencesMessage::AbbreviationRemove(
                            command.clone(),
                            abbreviation.clone(),
                        ))),
                    ),
                ),
            );
        }
    }
    if !any {
        let label = t!("prefs.abbreviations_none").to_string();
        add(
            Group::Search,
            label.clone(),
            setting_row(
                t!("prefs.abbreviations_title").to_string(),
                text(label).size(12).color(theme::muted_strong(dark)).into(),
            ),
        );
    }

    // ── Side panels ─────────────────────────────────────────────────────
    // (The two of the sidebar: one at a time, asking for one closes the
    // other. The database and API panels are the plugins' own.)
    for (label, on, panel) in [
        (
            t!("prefs.panel_files").to_string(),
            config.show_files_panel,
            crate::sidebar::Panel::Files,
        ),
        (
            t!("prefs.panel_projects").to_string(),
            config.show_projects_panel,
            crate::sidebar::Panel::Projects,
        ),
    ] {
        add(
            Group::Panels,
            label.clone(),
            setting_row(label, switch(dark, on, Message::TogglePanel(panel))),
        );
    }

    // ── Projects ────────────────────────────────────────────────────────
    let exclude = row![
        text_input("node_modules/, *.log, dist/", &state.exclude_draft)
            .on_input(|v| msg(PreferencesMessage::ExcludeTyped(v)))
            .on_submit(Message::SetProjectsExclude(state.exclude_draft.clone()))
            .padding([6, 10])
            .size(13)
            .width(260)
            .style(theme::Field::new(dark).style()),
        small_button(
            dark,
            t!("prefs.apply").to_string(),
            Some(Message::SetProjectsExclude(state.exclude_draft.clone())),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let label = t!("prefs.projects_exclude").to_string();
    add(
        Group::Projects,
        label.clone(),
        setting_row(label, exclude.into()),
    );
    // What Git ignores is hidden on the page of Git when it is installed;
    // without it, here.
    let hide = |dark: bool| {
        let label = t!("sidebar.hide_ignored").to_string();
        (
            label.clone(),
            setting_row(
                label,
                switch(
                    dark,
                    config.projects_hide_ignored,
                    Message::ToggleHideIgnored,
                ),
            ),
        )
    };
    let git_installed = ctx.plugins.get(crate::plugins::migrate::GIT).is_some();
    if git_installed {
        let (label, element) = hide(dark);
        add(Group::Git, label, element);
    } else {
        let (label, element) = hide(dark);
        add(Group::Projects, label, element);
    }

    // ── Scratch files ───────────────────────────────────────────────────
    let scratch = row![
        text_input(
            crate::scratch::default_dir().to_string_lossy().as_ref(),
            &state.scratch_draft,
        )
        .on_input(|v| msg(PreferencesMessage::ScratchDirTyped(v)))
        .on_submit(Message::SetScratchDir(state.scratch_draft.clone()))
        .padding([6, 10])
        .size(13)
        .width(300)
        .style(theme::Field::new(dark).style()),
        small_button(
            dark,
            t!("prefs.scratch_browse").to_string(),
            Some(Message::PickScratchDir),
        ),
        small_button(
            dark,
            t!("prefs.scratch_reset").to_string(),
            Some(Message::SetScratchDir(String::new())),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let label = t!("prefs.scratch_dir").to_string();
    add(
        Group::Scratch,
        label.clone(),
        setting_row(label, scratch.into()),
    );

    // ── Snippets ────────────────────────────────────────────────────────
    for (i, snippet) in config.snippets.iter().enumerate() {
        let first_line = snippet.body.lines().next().unwrap_or_default().to_string();
        let shown = if snippet.languages.is_empty() {
            snippet.trigger.clone()
        } else {
            format!("{}  ({})", snippet.trigger, snippet.languages.join(", "))
        };
        let muted = if snippet.enabled {
            theme::muted_strong(dark)
        } else {
            theme::muted_strong(dark).scale_alpha(0.5)
        };
        add(
            Group::Snippets,
            format!("{} {}", snippet.trigger, first_line),
            setting_row(
                shown,
                row![
                    text(first_line).size(12).font(theme::MONO).color(muted),
                    switch(
                        dark,
                        snippet.enabled,
                        msg(PreferencesMessage::SnippetEnabled(i, !snippet.enabled)),
                    ),
                    small_button(
                        dark,
                        t!("prefs.remove").to_string(),
                        Some(msg(PreferencesMessage::SnippetRemove(i))),
                    ),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
            ),
        );
    }
    let can_add =
        !state.snippet_trigger.trim().is_empty() && !state.snippet_body.text().trim().is_empty();
    let form = column![
        row![
            text_input(&t!("prefs.snippet_trigger"), &state.snippet_trigger)
                .on_input(|v| msg(PreferencesMessage::SnippetTrigger(v)))
                .padding([6, 10])
                .size(13)
                .width(140)
                .style(theme::Field::new(dark).style()),
            text_input(&t!("prefs.snippet_languages"), &state.snippet_languages)
                .on_input(|v| msg(PreferencesMessage::SnippetLanguages(v)))
                .padding([6, 10])
                .size(13)
                .width(Length::Fill)
                .style(theme::Field::new(dark).style()),
        ]
        .spacing(8),
        text_editor(&state.snippet_body)
            .on_action(|a| msg(PreferencesMessage::SnippetBody(a)))
            .placeholder(t!("prefs.snippet_body").to_string())
            .size(13)
            .height(90)
            .padding(8),
        row![
            Space::new().width(Length::Fill),
            small_button(
                dark,
                t!("prefs.snippet_add").to_string(),
                can_add.then(|| msg(PreferencesMessage::SnippetAdd)),
            ),
        ],
    ]
    .spacing(8);
    let label = t!("prefs.snippet_new").to_string();
    add(
        Group::Snippets,
        label.clone(),
        container(form).padding([12, 16]).width(Length::Fill).into(),
    );

    // ── Trusted folders ─────────────────────────────────────────────────
    for folder in &config.trusted_folders {
        add(
            Group::Trust,
            folder.clone(),
            setting_row(
                folder.clone(),
                small_button(
                    dark,
                    t!("prefs.trust_remove").to_string(),
                    Some(msg(PreferencesMessage::UntrustFolder(folder.clone()))),
                ),
            ),
        );
    }
    if config.trusted_folders.is_empty() {
        add(
            Group::Trust,
            t!("prefs.trust_none").to_string(),
            setting_row(
                t!("prefs.trust_title").to_string(),
                text(t!("prefs.trust_none").to_string())
                    .size(12)
                    .color(theme::muted_strong(dark))
                    .into(),
            ),
        );
    }

    // ── History ─────────────────────────────────────────────────────────
    for (label, filled, message) in [
        (
            t!("prefs.history_recent_files").to_string(),
            !config.recent_files.is_empty(),
            PreferencesMessage::ClearHistory(HistoryKind::RecentFiles),
        ),
        (
            t!("prefs.history_find").to_string(),
            !config.find_history.is_empty(),
            PreferencesMessage::ClearHistory(HistoryKind::Find),
        ),
        (
            t!("prefs.history_replace").to_string(),
            !config.replace_history.is_empty(),
            PreferencesMessage::ClearHistory(HistoryKind::Replace),
        ),
        (
            t!("prefs.history_hosts").to_string(),
            !config.remote_hosts.is_empty(),
            PreferencesMessage::ClearHistory(HistoryKind::RemoteHosts),
        ),
    ] {
        add(
            Group::History,
            label.clone(),
            setting_row(
                label,
                small_button(
                    dark,
                    t!("prefs.clear").to_string(),
                    filled.then(|| msg(message)),
                ),
            ),
        );
    }

    // ── Language ────────────────────────────────────────────────────────
    let label = t!("prefs.language").to_string();
    add(Group::Language, label, super::language_row(config, dark));

    // ── Appearance ──────────────────────────────────────────────────────
    let seg = |label: String, is_dark: bool| {
        button(text(label).size(12))
            .padding([3, 12])
            .on_press(Message::ThemeChanged(is_dark))
            .style(
                (theme::Segment {
                    dark,
                    active: dark == is_dark,
                    radius: theme::R_SM,
                })
                .style(),
            )
    };
    let control = container(
        row![
            seg(t!("toolbar.dark").to_string(), true),
            seg(t!("toolbar.light").to_string(), false)
        ]
        .spacing(2),
    )
    .padding(2)
    .style(theme::field(dark, false, theme::R_ROW));
    let label = t!("prefs.theme").to_string();
    add(
        Group::Appearance,
        label.clone(),
        setting_row(label, control.into()),
    );
    let (label, element) = toggle(
        t!("prefs.high_contrast").to_string(),
        config.high_contrast,
        PreferencesMessage::HighContrastToggled,
    );
    add(Group::Appearance, label, element);

    // ── The Git plugin: its tab in Search everywhere ────────────────────
    let (label, element) = toggle(
        t!("prefs.everywhere_plugins").to_string(),
        config.everywhere_git,
        PreferencesMessage::EverywhereGitToggled,
    );
    add(Group::Git, label, element);

    // ── The spelling plugin ─────────────────────────────────────────────
    let (label, element) = toggle(
        t!("prefs.spell_check").to_string(),
        config.spell_check,
        PreferencesMessage::SpellCheckToggled,
    );
    add(Group::Spelling, label, element);
    let auto = t!("spell.language_auto").to_string();
    let mut languages = vec![auto.clone()];
    languages.extend(ctx.spell_choices.iter().cloned());
    let chosen = if config.spell_language.is_empty() {
        auto.clone()
    } else {
        config.spell_language.clone()
    };
    let select = pick_list(languages, Some(chosen), move |picked| {
        Message::SpellLanguage(if picked == auto {
            String::new()
        } else {
            picked
        })
    })
    .padding([6, 12])
    .width(150)
    .text_size(13)
    .style(theme::Select { dark }.style())
    .menu_style(theme::Select { dark }.menu_style());
    let label = t!("prefs.spell_language").to_string();
    add(
        Group::Spelling,
        label.clone(),
        setting_row(label, select.into()),
    );
    // Where the dictionaries come from: there is nothing to add here, they are
    // the ones of the machine.
    let none = ctx.spell_choices.is_empty();
    let help = if none {
        format!(
            "{} {}",
            t!("prefs.spell_dictionaries_none"),
            t!("prefs.spell_dictionaries_help")
        )
    } else {
        t!("prefs.spell_dictionaries_help").to_string()
    };
    add(
        Group::Spelling,
        t!("prefs.spell_dictionaries_title").to_string(),
        container(text(help).size(12).color(theme::muted_strong(dark)))
            .padding([12, 16])
            .width(Length::Fill)
            .into(),
    );

    out
}

/// What a Clear button empties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    RecentFiles,
    Find,
    Replace,
    RemoteHosts,
}

/// The settings of some groups, as blocks (a title, a Reset button when the
/// group has one, a card of rows), in the order of `groups`.
pub fn blocks<'a>(
    settings: Vec<Setting<'a>>,
    groups: &[Group],
    dark: bool,
    place_in_title: bool,
) -> Element<'a, Message> {
    let mut settings = settings;
    let mut page = column![].spacing(22);
    for group in groups {
        let rows: Vec<Element<'a, Message>> = {
            let (mine, rest): (Vec<Setting<'a>>, Vec<Setting<'a>>) =
                settings.into_iter().partition(|s| s.group == *group);
            settings = rest;
            mine.into_iter().map(|s| s.element).collect()
        };
        if rows.is_empty() {
            continue;
        }
        let title = if place_in_title {
            group.place()
        } else {
            group.title()
        };
        let mut head = row![text(title)
            .size(12)
            .font(theme::UI_SEMIBOLD)
            .color(theme::muted_strong(dark))]
        .align_y(Alignment::Center)
        .push(Space::new().width(Length::Fill));
        if group.resettable() && !place_in_title {
            head = head.push(
                button(
                    text(t!("prefs.reset").to_string())
                        .size(11.5)
                        .color(theme::muted_strong(dark)),
                )
                .padding([2, 8])
                .on_press(msg(PreferencesMessage::ResetGroup(*group)))
                .style(
                    (theme::GhostButton {
                        dark,
                        active: false,
                    })
                    .style(),
                ),
            );
        }
        page = page.push(column![head, card_of_rows(dark, rows)].spacing(8));
    }
    page.into()
}

/// The settings whose label (or whose block) has every word of `filter`,
/// accents and capitals apart.
pub fn matching<'a>(settings: Vec<Setting<'a>>, filter: &str) -> Vec<Setting<'a>> {
    let words: Vec<String> = filter.split_whitespace().map(plain).collect();
    settings
        .into_iter()
        .filter(|s| {
            let hay = format!("{} {}", plain(&s.label), plain(&s.group.title()));
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

/// The results of a search: every setting whose label (or whose block) has all
/// the words, by block, with the place of the block.
pub fn found<'a>(settings: Vec<Setting<'a>>, filter: &str, dark: bool) -> Element<'a, Message> {
    let hits = matching(settings, filter);
    if hits.is_empty() {
        return text(t!("prefs.search_none").to_string())
            .size(13)
            .color(theme::muted_strong(dark))
            .into();
    }
    let mut order: Vec<Group> = Vec::new();
    for s in &hits {
        if !order.contains(&s.group) {
            order.push(s.group);
        }
    }
    blocks(hits, &order, dark, true)
}

/// How snippets work and how to make one, above the list.
pub fn snippets_help(dark: bool) -> Element<'static, Message> {
    let body = |text_: String| text(text_).size(13).color(theme::muted_strong(dark));
    container(
        column![
            text(t!("prefs.snippets_help_title").to_string())
                .size(13)
                .font(theme::UI_SEMIBOLD),
            body(t!("prefs.snippets_help_what").to_string()),
            body(t!("prefs.snippets_help_use").to_string()),
            body(t!("prefs.snippets_help_make").to_string()),
            body(t!("prefs.snippets_help_syntax").to_string()),
        ]
        .spacing(8),
    )
    .padding([14, 16])
    .width(Length::Fill)
    .style(theme::field(dark, false, theme::R_ROW))
    .into()
}
