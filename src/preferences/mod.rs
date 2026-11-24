//! The Preferences window: a navigation column on the left (Editor,
//! Appearance, Shortcuts, Language, Updates) and settings cards on the right.

use iced::{
    widget::{
        button, column, container, pick_list, row, scrollable, slider, text, text_input, Space,
    },
    Alignment, Element, Length, Padding,
};

mod groups;
pub use groups::{Group, HistoryKind};
mod plugins;

use crate::{
    app::Message,
    config::{Config, ShortcutTarget},
    theme,
};

/// What the pointer took hold of on the Preferences window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefsGrab {
    /// The title bar: the window moves.
    Move,
    /// The right edge, the bottom edge, the corner: the window is resized.
    Right,
    Bottom,
    Corner,
}

#[derive(Debug, Clone)]
pub enum PreferencesMessage {
    PageSelected(PrefsPage),
    /// The page of the settings of a plugin (its id).
    PluginPageSelected(String),
    ExcludeTyped(String),
    /// The search of the shortcuts, as typed.
    ShortcutFilter(String),
    FontSizeChanged(f32),
    TabWidthChanged(usize),
    UseSpacesToggled(bool),
    ShowLineNumbersToggled(bool),
    AutoIndentToggled(bool),
    CompletionToggled(bool),
    CompletionAutoToggled(bool),
    /// A completion setting of a language (`rust.postfix`, …).
    CompletionOption(String, bool),
    EverywhereTextToggled(bool),
    EverywhereGitToggled(bool),
    AutocompleteBracketsToggled(bool),
    AutocompleteQuotesToggled(bool),
    HighlightCurrentLineToggled(bool),
    ShowPageGuideToggled(bool),
    ShowIndentGuidesToggled(bool),
    DetectIndentToggled(bool),
    FormatOnSaveToggled(bool),
    SpellCheckToggled(bool),
    HighContrastToggled(bool),
    ShowWhitespaceToggled(bool),
    MatchBracketsToggled(bool),
    HighlightWordToggled(bool),
    AutoSaveToggled(bool),
    PageGuideColumnChanged(usize),
    ShowMinimapToggled(bool),
    WordWrapToggled(bool),
    LocaleChanged(String),
    /// The scratch folder being typed (applied on Enter).
    ScratchDirTyped(String),
    /// The font family being typed, and the one that is set (empty: default).
    FontFamilyTyped(String),
    FontFamilySet(String),
    TrimTrailingToggled(bool),
    FinalNewlineToggled(bool),
    DefaultEncodingChanged(String),
    DefaultLineEndingChanged(String),
    /// A block of settings put back to its defaults.
    ResetGroup(Group),
    /// An abbreviation of an action removed: the action, the abbreviation.
    AbbreviationRemove(String, String),
    SnippetTrigger(String),
    SnippetLanguages(String),
    SnippetBody(iced::widget::text_editor::Action),
    SnippetAdd,
    SnippetRemove(usize),
    /// A folder whose programs may run, no longer trusted.
    UntrustFolder(String),
    ClearHistory(HistoryKind),
    /// What the search of the settings looks for.
    SettingsSearch(String),
}

/// The pages of the navigation column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefsPage {
    Editor,
    Appearance,
    Shortcuts,
    Lint,
    Servers,
    /// The settings of a plugin: which one is `PreferencesState::plugin_page`.
    /// Not in the list of the pages: each plugin has its own entry.
    PluginSettings,
    Plugins,
}

impl PrefsPage {
    pub(crate) const ALL: [PrefsPage; 6] = [
        PrefsPage::Editor,
        PrefsPage::Appearance,
        PrefsPage::Shortcuts,
        PrefsPage::Lint,
        PrefsPage::Servers,
        PrefsPage::Plugins,
    ];

    pub(crate) fn label(self) -> String {
        match self {
            PrefsPage::Editor => t!("prefs.nav_editor"),
            PrefsPage::Appearance => t!("prefs.nav_appearance"),
            PrefsPage::Shortcuts => t!("prefs.nav_shortcuts"),
            PrefsPage::Lint => t!("prefs.nav_lint"),
            PrefsPage::Servers => t!("prefs.nav_servers"),
            PrefsPage::PluginSettings | PrefsPage::Plugins => t!("prefs.nav_plugins"),
        }
        .to_string()
    }

    /// Whether the page is in the preferences: Lint only once a plugin
    /// that lints is installed.
    pub(crate) fn shown(self, plugins: &crate::plugins::Plugins) -> bool {
        self != PrefsPage::Lint || plugins.linter().is_some()
    }

    fn title(self) -> String {
        match self {
            PrefsPage::Editor => t!("prefs.editor").to_string(),
            other => other.label(),
        }
    }
}

/// An entry of the language select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocaleChoice {
    code: &'static str,
    name: &'static str,
}

impl std::fmt::Display for LocaleChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

// The language names stay in their own language.
const LOCALES: [LocaleChoice; 2] = [
    LocaleChoice {
        code: "en",
        name: "English",
    },
    LocaleChoice {
        code: "fr",
        name: "Français",
    },
];

/// What the Servers page shows.
#[derive(Debug, Default)]
pub struct ServerStatus {
    /// Programs of the language servers that can be run.
    pub ready: std::collections::HashSet<&'static str>,
    /// Of those, the ones Ardoise installed and can remove.
    pub removable: std::collections::HashSet<&'static str>,
    /// Programs being installed.
    pub installing: std::collections::HashSet<&'static str>,
    /// Programs being removed.
    pub removing: std::collections::HashSet<&'static str>,
}

pub struct PreferencesState {
    pub page: PrefsPage,
    /// The plugin whose settings are the page, on `PrefsPage::PluginSettings`.
    pub plugin_page: Option<String>,
    pub scratch_draft: String,
    /// The names left out of the trees, as typed.
    pub exclude_draft: String,
    /// What the search of the shortcuts looks for.
    pub shortcut_filter: String,
    /// What the search of the settings looks for.
    pub settings_filter: String,
    /// The font family being typed.
    pub font_draft: String,
    /// The snippet being written.
    pub snippet_trigger: String,
    pub snippet_languages: String,
    pub snippet_body: iced::widget::text_editor::Content,
}

impl PreferencesState {
    pub fn from_config(config: &Config) -> Self {
        Self {
            page: PrefsPage::Editor,
            plugin_page: None,
            scratch_draft: config.scratch_dir.clone(),
            exclude_draft: config.projects_exclude.join(", "),
            shortcut_filter: String::new(),
            settings_filter: String::new(),
            font_draft: config.font_family.clone(),
            snippet_trigger: String::new(),
            snippet_languages: String::new(),
            snippet_body: iced::widget::text_editor::Content::new(),
        }
    }

    /// Applies a change. Returns whether the config changed (page
    /// navigation doesn't touch it).
    pub fn update(&mut self, msg: PreferencesMessage, config: &mut Config) -> bool {
        match msg {
            PreferencesMessage::ExcludeTyped(v) => {
                self.exclude_draft = v;
                return false;
            }
            PreferencesMessage::ShortcutFilter(v) => {
                self.shortcut_filter = v;
                return false;
            }
            PreferencesMessage::PageSelected(page) => {
                self.page = page;
                return false;
            }
            PreferencesMessage::PluginPageSelected(id) => {
                self.page = PrefsPage::PluginSettings;
                self.plugin_page = Some(id);
                return false;
            }
            PreferencesMessage::FontSizeChanged(v) => {
                config.font_size = v;
            }
            PreferencesMessage::TabWidthChanged(v) => {
                config.tab_width = v;
            }
            PreferencesMessage::UseSpacesToggled(v) => {
                config.use_spaces = v;
            }
            PreferencesMessage::ShowLineNumbersToggled(v) => {
                config.show_line_numbers = v;
            }
            PreferencesMessage::AutoIndentToggled(v) => {
                config.auto_indent = v;
            }
            PreferencesMessage::CompletionToggled(v) => {
                config.completion_enabled = v;
            }
            PreferencesMessage::CompletionAutoToggled(v) => {
                config.completion_auto = v;
            }
            PreferencesMessage::CompletionOption(key, v) => {
                config.completion_options.insert(key, v);
            }
            PreferencesMessage::EverywhereTextToggled(v) => {
                config.everywhere_text = v;
            }
            PreferencesMessage::EverywhereGitToggled(v) => {
                config.everywhere_git = v;
            }
            PreferencesMessage::AutocompleteBracketsToggled(v) => {
                config.autocomplete_brackets = v;
            }
            PreferencesMessage::AutocompleteQuotesToggled(v) => {
                config.autocomplete_quotes = v;
            }
            PreferencesMessage::HighlightCurrentLineToggled(v) => {
                config.highlight_current_line = v;
            }
            PreferencesMessage::PageGuideColumnChanged(v) => {
                config.page_guide_column = v;
            }
            PreferencesMessage::ShowPageGuideToggled(v) => {
                config.show_page_guide = v;
            }
            PreferencesMessage::HighContrastToggled(v) => {
                config.high_contrast = v;
            }
            PreferencesMessage::SpellCheckToggled(v) => {
                config.spell_check = v;
            }
            PreferencesMessage::FormatOnSaveToggled(v) => {
                config.format_on_save = v;
            }
            PreferencesMessage::DetectIndentToggled(v) => {
                config.detect_indent = v;
            }
            PreferencesMessage::ShowIndentGuidesToggled(v) => {
                config.show_indent_guides = v;
            }
            PreferencesMessage::ShowWhitespaceToggled(v) => {
                config.show_whitespace = v;
            }
            PreferencesMessage::HighlightWordToggled(v) => {
                config.highlight_word = v;
            }
            PreferencesMessage::MatchBracketsToggled(v) => {
                config.match_brackets = v;
            }
            PreferencesMessage::AutoSaveToggled(v) => {
                config.auto_save = v;
            }
            PreferencesMessage::ShowMinimapToggled(v) => {
                config.show_minimap = v;
            }
            PreferencesMessage::WordWrapToggled(v) => {
                config.word_wrap = v;
            }
            PreferencesMessage::ScratchDirTyped(v) => {
                self.scratch_draft = v;
                return false;
            }
            PreferencesMessage::FontFamilyTyped(v) => {
                self.font_draft = v;
                return false;
            }
            PreferencesMessage::FontFamilySet(v) => {
                config.font_family = v.trim().to_string();
                self.font_draft = config.font_family.clone();
            }
            PreferencesMessage::TrimTrailingToggled(v) => {
                config.trim_trailing_whitespace = v;
            }
            PreferencesMessage::FinalNewlineToggled(v) => {
                config.final_newline_on_save = v;
            }
            PreferencesMessage::DefaultEncodingChanged(v) => {
                config.default_encoding = if v == crate::tabs::Encoding::Utf8.label() {
                    String::new()
                } else {
                    v
                };
            }
            PreferencesMessage::DefaultLineEndingChanged(v) => {
                config.default_line_ending = if v == "LF" { String::new() } else { v };
            }
            PreferencesMessage::ResetGroup(group) => {
                let d = Config::default();
                match group {
                    Group::Display => {
                        config.font_size = d.font_size;
                        config.font_family = d.font_family;
                        self.font_draft = config.font_family.clone();
                        config.show_line_numbers = d.show_line_numbers;
                        config.highlight_current_line = d.highlight_current_line;
                        config.show_minimap = d.show_minimap;
                        config.show_page_guide = d.show_page_guide;
                        config.page_guide_column = d.page_guide_column;
                        config.show_indent_guides = d.show_indent_guides;
                        config.show_whitespace = d.show_whitespace;
                        config.word_wrap = d.word_wrap;
                    }
                    Group::Indentation => {
                        config.tab_width = d.tab_width;
                        config.use_spaces = d.use_spaces;
                        config.auto_indent = d.auto_indent;
                        config.detect_indent = d.detect_indent;
                    }
                    Group::Typing => {
                        config.autocomplete_brackets = d.autocomplete_brackets;
                        config.autocomplete_quotes = d.autocomplete_quotes;
                        config.match_brackets = d.match_brackets;
                        config.highlight_word = d.highlight_word;
                    }
                    Group::Completion => {
                        config.completion_enabled = d.completion_enabled;
                        config.completion_auto = d.completion_auto;
                        config.completion_options = d.completion_options;
                    }
                    Group::Files => {
                        config.auto_save = d.auto_save;
                        config.format_on_save = d.format_on_save;
                        config.trim_trailing_whitespace = d.trim_trailing_whitespace;
                        config.final_newline_on_save = d.final_newline_on_save;
                        config.default_encoding = d.default_encoding;
                        config.default_line_ending = d.default_line_ending;
                    }
                    Group::Search => {
                        config.everywhere_text = d.everywhere_text;
                    }
                    // The lists the user wrote have nothing to go back to.
                    _ => return false,
                }
            }
            PreferencesMessage::AbbreviationRemove(command, abbreviation) => {
                if let Some(list) = config.action_abbreviations.get_mut(&command) {
                    list.retain(|a| *a != abbreviation);
                    if list.is_empty() {
                        config.action_abbreviations.remove(&command);
                    }
                }
            }
            PreferencesMessage::SnippetTrigger(v) => {
                self.snippet_trigger = v;
                return false;
            }
            PreferencesMessage::SnippetLanguages(v) => {
                self.snippet_languages = v;
                return false;
            }
            PreferencesMessage::SnippetBody(action) => {
                self.snippet_body.perform(action);
                return false;
            }
            PreferencesMessage::SnippetAdd => {
                let trigger = self.snippet_trigger.trim().to_string();
                let body = self.snippet_body.text().trim_end().to_string();
                if trigger.is_empty() || body.is_empty() {
                    return false;
                }
                let languages = self
                    .snippet_languages
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .map(|l| l.trim().trim_start_matches('.').to_lowercase())
                    .filter(|l| !l.is_empty())
                    .collect();
                config.snippets.push(crate::config::UserSnippet {
                    trigger,
                    body,
                    languages,
                });
                self.snippet_trigger.clear();
                self.snippet_languages.clear();
                self.snippet_body = iced::widget::text_editor::Content::new();
            }
            PreferencesMessage::SnippetRemove(i) => {
                if i < config.snippets.len() {
                    config.snippets.remove(i);
                }
            }
            PreferencesMessage::UntrustFolder(folder) => {
                config.trusted_folders.retain(|f| *f != folder);
                crate::trust::set_all(&config.trusted_folders);
            }
            PreferencesMessage::ClearHistory(kind) => match kind {
                HistoryKind::RecentFiles => config.recent_files.clear(),
                HistoryKind::Find => config.find_history.clear(),
                HistoryKind::Replace => config.replace_history.clear(),
                HistoryKind::RemoteHosts => config.remote_hosts.clear(),
            },
            PreferencesMessage::SettingsSearch(v) => {
                self.settings_filter = v;
                return false;
            }
            PreferencesMessage::LocaleChanged(v) => {
                config.locale = v.clone();
                rust_i18n::set_locale(&v);
            }
        }
        true
    }

    /// The whole window: title row, navigation column, page.
    pub fn view<'a>(
        &'a self,
        config: &Config,
        capturing: Option<ShortcutTarget>,
        servers: &ServerStatus,
        plugin_state: &'a crate::plugins::Plugins,
        spell_choices: &[String],
    ) -> Element<'a, Message> {
        let dark = config.dark_mode;
        let p = theme::palette(dark);

        let title_row = crate::popin::header(
            dark,
            crate::popin::Head::new("sliders-horizontal", t!("prefs.window_title").to_string())
                .sub(t!("prefs.subtitle").to_string()),
            Some(Message::TogglePreferences),
        );
        // The window is moved by its title bar.
        let title_row = iced::widget::mouse_area(title_row)
            .on_press(Message::PrefsDragStart(PrefsGrab::Move))
            .interaction(iced::mouse::Interaction::Grab);

        // The plugins that have settings have a page each; a page that is
        // no longer there (its plugin removed) is the editor's.
        let plugin_pages = plugin_state.settings_pages();
        let current_plugin = self
            .plugin_page
            .as_ref()
            .and_then(|id| plugin_pages.iter().find(|(page_id, _)| page_id == id));
        let shown = match self.page {
            PrefsPage::PluginSettings if current_plugin.is_some() => PrefsPage::PluginSettings,
            PrefsPage::PluginSettings => PrefsPage::Editor,
            other if other.shown(plugin_state) => other,
            _ => PrefsPage::Editor,
        };
        let nav_entry = |label: String, msg: PreferencesMessage, selected: bool| {
            button(text(label).size(13))
                .padding([8, 12])
                .width(Length::Fill)
                .on_press(Message::Preferences(msg))
                .style((theme::SidebarRow { dark, selected }).style())
                .into()
        };
        let mut nav_items: Vec<Element<Message>> = Vec::new();
        for page in PrefsPage::ALL
            .iter()
            .filter(|page| page.shown(plugin_state))
        {
            // The settings of the plugins come before the list of the plugins.
            if *page == PrefsPage::Plugins {
                for (id, name) in &plugin_pages {
                    let selected = shown == PrefsPage::PluginSettings
                        && current_plugin.is_some_and(|(page_id, _)| page_id == id);
                    nav_items.push(nav_entry(
                        name.clone(),
                        PreferencesMessage::PluginPageSelected(id.clone()),
                        selected,
                    ));
                }
            }
            nav_items.push(nav_entry(
                page.label(),
                PreferencesMessage::PageSelected(*page),
                shown == *page,
            ));
        }

        let search = text_input(&t!("prefs.search_placeholder"), &self.settings_filter)
            .on_input(|v| Message::Preferences(PreferencesMessage::SettingsSearch(v)))
            .padding([7, 10])
            .size(13)
            .style(theme::Field::new(dark).style());
        let nav = container(column![search, column(nav_items).spacing(2)].spacing(14))
            .padding([16, 10])
            .width(210)
            .height(Length::Fill)
            .style(move |_: &iced::Theme| container::Style {
                text_color: Some(p.text),
                background: Some(iced::Background::Color(p.tabs)),
                border: iced::Border {
                    color: iced::Color::TRANSPARENT,
                    width: 0.0,
                    radius: iced::border::Radius::default().bottom_left(theme::R_POPIN - 1.0),
                },
                shadow: iced::Shadow::default(),
                ..container::Style::default()
            });

        // Every setting of the editor as a row, then the page (or the search)
        // takes the blocks it shows.
        let settings = groups::all(
            self,
            &groups::Context {
                config,
                dark,
                plugins: plugin_state,
                spell_choices,
            },
        );
        let searching = !self.settings_filter.trim().is_empty();
        let page: Element<Message> = if searching {
            groups::found(settings, &self.settings_filter, dark)
        } else {
            match shown {
                PrefsPage::Editor => groups::blocks(
                    settings,
                    &[
                        Group::Display,
                        Group::Indentation,
                        Group::Typing,
                        Group::Files,
                        Group::Search,
                        Group::Panels,
                        Group::Projects,
                        Group::Scratch,
                        Group::Snippets,
                        Group::Trust,
                        Group::History,
                        Group::Language,
                    ],
                    dark,
                    false,
                ),
                PrefsPage::Appearance => {
                    groups::blocks(settings, &[Group::Appearance], dark, false)
                }
                PrefsPage::Shortcuts => {
                    shortcuts_list(dark, config, capturing, plugin_state, &self.shortcut_filter)
                }
                PrefsPage::Lint => lint_page(config, dark, plugin_state),
                PrefsPage::Servers => column![
                    groups::blocks(settings, &[Group::Completion], dark, false),
                    self.servers_page(dark, servers),
                ]
                .spacing(22)
                .into(),
                PrefsPage::PluginSettings => match current_plugin {
                    // What the editor keeps for the plugin, then the plugin's own.
                    Some((id, _)) => {
                        let mut page = column![].spacing(18);
                        let own: &[Group] = match id.as_str() {
                            crate::plugins::migrate::GIT => &[Group::Git],
                            crate::plugins::migrate::SPELL => &[Group::Spelling],
                            _ => &[],
                        };
                        if !own.is_empty() {
                            page = page.push(groups::blocks(settings, own, dark, false));
                        }
                        if let Some(body) = plugin_settings_body(plugin_state, id, dark) {
                            page = page.push(body);
                        }
                        page.into()
                    }
                    None => Space::new().into(),
                },
                PrefsPage::Plugins => plugins::page(plugin_state, dark),
            }
        };

        let page_title = if searching {
            t!("prefs.search_results").to_string()
        } else {
            match (shown, current_plugin) {
                (PrefsPage::PluginSettings, Some((_, name))) => name.clone(),
                _ => shown.title(),
            }
        };
        let content = scrollable(
            column![text(page_title).size(22).font(theme::UI_BOLD), page,]
                .spacing(18)
                .padding(Padding {
                    top: 26.0,
                    right: 40.0,
                    bottom: 26.0,
                    left: 40.0,
                })
                .width(Length::Fill),
        )
        .style(theme::Scrollbar { dark }.style())
        .width(Length::Fill)
        .height(Length::Fill);

        container(column![
            title_row,
            container(Space::new().height(1))
                .width(Length::Fill)
                .style(theme::popin_rule(dark)),
            row![nav, content].height(Length::Fill),
        ])
        .style(theme::popin_card(dark))
        .into()
    }

    /// The language servers: installed ones give exact answers to Ctrl+click
    /// on a name; without one the editor searches by patterns.
    fn servers_page(&self, dark: bool, status: &ServerStatus) -> Element<'_, Message> {
        use crate::lsp_install::{Installer, SERVERS};
        let p = theme::palette(dark);
        let note = |label: String| text(label).size(12).color(p.muted);
        let rows = SERVERS
            .iter()
            .map(|info| {
                let name = format!("{} ({})", info.label, info.extensions.join(" "));
                let control: Element<Message> = if status.removing.contains(info.binary) {
                    note(t!("prefs.server_uninstalling").to_string()).into()
                } else if status.ready.contains(info.binary) {
                    let installed = note(t!("prefs.server_installed").to_string());
                    if status.removable.contains(info.binary) {
                        row![
                            installed,
                            button(text(t!("prefs.server_uninstall").to_string()).size(12))
                                .padding([6, 12])
                                .on_press(Message::UninstallServer(info.binary))
                                .style((theme::Outline { dark }).style()),
                        ]
                        .spacing(12)
                        .align_y(Alignment::Center)
                        .into()
                    } else {
                        installed.into()
                    }
                } else if status.installing.contains(info.binary) {
                    note(t!("prefs.server_installing").to_string()).into()
                } else if let Installer::Manual(hint) = info.installer {
                    note(hint.to_string()).font(theme::MONO).into()
                } else {
                    button(text(t!("prefs.server_install").to_string()).size(12))
                        .padding([6, 12])
                        .on_press(Message::InstallServer(info.binary))
                        .style((theme::Outline { dark }).style())
                        .into()
                };
                setting_row(name, control)
            })
            .collect();
        column![
            text(t!("prefs.servers_intro").to_string())
                .size(12)
                .color(p.muted),
            card_of_rows(dark, rows),
        ]
        .spacing(12)
        .into()
    }
}

/// The row of the language of the interface.
fn language_row(config: &Config, dark: bool) -> Element<'static, Message> {
    let selected = LOCALES.iter().copied().find(|l| l.code == config.locale);
    let select = pick_list(LOCALES, selected, |choice| {
        Message::Preferences(PreferencesMessage::LocaleChanged(choice.code.to_string()))
    })
    .padding([6, 12])
    .width(150)
    .text_size(13)
    .style(theme::Select { dark }.style())
    .menu_style(theme::Select { dark }.menu_style());
    setting_row(t!("prefs.language").to_string(), select.into())
}

/// The few settings the welcome popin asks: language, theme, size of the
/// text, saving, wrapping. They are the ones of the Preferences, under the
/// same messages.
pub(crate) fn basics<'a>(config: &Config, dark: bool) -> Element<'a, Message> {
    let selected = LOCALES.iter().copied().find(|l| l.code == config.locale);
    let language = pick_list(LOCALES, selected, |choice| {
        Message::Preferences(PreferencesMessage::LocaleChanged(choice.code.to_string()))
    })
    .padding([6, 12])
    .width(150)
    .text_size(13)
    .style(theme::Select { dark }.style())
    .menu_style(theme::Select { dark }.menu_style());
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
    let appearance = container(
        row![
            seg(t!("toolbar.dark").to_string(), true),
            seg(t!("toolbar.light").to_string(), false)
        ]
        .spacing(2),
    )
    .padding(2)
    .style(theme::field(dark, false, theme::R_ROW));
    let font_size = row![
        slider(8.0..=32.0, config.font_size, |v| {
            Message::Preferences(PreferencesMessage::FontSizeChanged(v.round()))
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
    card_of_rows(
        dark,
        vec![
            setting_row(t!("prefs.language").to_string(), language.into()),
            setting_row(t!("prefs.theme").to_string(), appearance.into()),
            setting_row(t!("prefs.font_size").to_string(), font_size.into()),
            setting_row(
                t!("prefs.auto_save").to_string(),
                switch(
                    dark,
                    config.auto_save,
                    Message::Preferences(PreferencesMessage::AutoSaveToggled(!config.auto_save)),
                ),
            ),
            setting_row(
                t!("prefs.word_wrap").to_string(),
                switch(
                    dark,
                    config.word_wrap,
                    Message::Preferences(PreferencesMessage::WordWrapToggled(!config.word_wrap)),
                ),
            ),
        ],
    )
}

/// A label on the left, its control on the right.
fn setting_row<'a>(label: String, control: Element<'a, Message>) -> Element<'a, Message> {
    row![text(label).size(13).width(Length::Fill), control]
        .spacing(16)
        .padding([12, 16])
        .align_y(Alignment::Center)
        .into()
}

/// Rows on a bordered card, separated by hairlines.
pub(crate) fn card_of_rows<'a>(
    dark: bool,
    rows: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut col = column![];
    let last = rows.len().saturating_sub(1);
    for (i, row_el) in rows.into_iter().enumerate() {
        col = col.push(row_el);
        if i != last {
            col = col.push(
                container(Space::new().height(1))
                    .width(Length::Fill)
                    .style(theme::rule(dark)),
            );
        }
    }
    container(col)
        .width(Length::Fill)
        .style(theme::settings_card(dark))
        .into()
}

/// A 36×20 switch with a 14px knob.
pub fn switch(dark: bool, on: bool, message: Message) -> Element<'static, Message> {
    let knob = container(Space::new().width(14).height(14)).style(theme::switch_knob(dark, on));
    button(row![Space::new().width(if on { 17.0 } else { 0.0 }), knob])
        .padding(3)
        .width(36)
        .height(20)
        .on_press(message)
        .style((theme::SwitchTrack { dark, on }).style())
        .into()
}

/// The configurable shortcuts: one row per action, click a key to rebind it.
/// Used by the Preferences page and by Help > Configure shortcuts.
/// Whether a shortcut is listed: the ones of a plugin only once it is
/// installed.
pub(crate) fn shortcut_shown(target: ShortcutTarget, plugins: &crate::plugins::Plugins) -> bool {
    PLUGIN_SHORTCUTS
        .iter()
        .find(|(_, targets)| targets.contains(&target))
        .is_none_or(|(id, _)| plugins.get(id).is_some())
}

/// Every shortcut that can be rebound: the action, its label, its keys.
fn shortcut_entries(sc: &crate::config::Shortcuts) -> Vec<(ShortcutTarget, String, String)> {
    vec![
        (
            ShortcutTarget::Undo,
            t!("shortcuts.undo").to_string(),
            sc.undo.display(),
        ),
        (
            ShortcutTarget::Redo,
            t!("shortcuts.redo").to_string(),
            sc.redo.display(),
        ),
        (
            ShortcutTarget::DuplicateLine,
            t!("shortcuts.duplicate_line").to_string(),
            sc.duplicate_line.display(),
        ),
        (
            ShortcutTarget::MoveLineUp,
            t!("shortcuts.move_line_up").to_string(),
            sc.move_line_up.display(),
        ),
        (
            ShortcutTarget::MoveLineDown,
            t!("shortcuts.move_line_down").to_string(),
            sc.move_line_down.display(),
        ),
        (
            ShortcutTarget::ToggleComment,
            t!("shortcuts.toggle_comment").to_string(),
            sc.toggle_comment.display(),
        ),
        (
            ShortcutTarget::DeleteLine,
            t!("shortcuts.delete_line").to_string(),
            sc.delete_line.display(),
        ),
        (
            ShortcutTarget::SpellFix,
            t!("shortcuts.spell_fix").to_string(),
            sc.spell_fix.display(),
        ),
        (
            ShortcutTarget::RunTask,
            t!("shortcuts.run_task").to_string(),
            sc.run_task.display(),
        ),
        (
            ShortcutTarget::RenameSymbol,
            t!("shortcuts.rename_symbol").to_string(),
            sc.rename_symbol.display(),
        ),
        (
            ShortcutTarget::AddNextOccurrence,
            t!("shortcuts.add_next_occurrence").to_string(),
            sc.add_next_occurrence.display(),
        ),
        (
            ShortcutTarget::SelectAllOccurrences,
            t!("shortcuts.select_all_occurrences").to_string(),
            sc.select_all_occurrences.display(),
        ),
        (
            ShortcutTarget::AddCursorAbove,
            t!("shortcuts.add_cursor_above").to_string(),
            sc.add_cursor_above.display(),
        ),
        (
            ShortcutTarget::AddCursorBelow,
            t!("shortcuts.add_cursor_below").to_string(),
            sc.add_cursor_below.display(),
        ),
        (
            ShortcutTarget::ToggleBookmark,
            t!("shortcuts.toggle_bookmark").to_string(),
            sc.toggle_bookmark.display(),
        ),
        (
            ShortcutTarget::NextBookmark,
            t!("shortcuts.next_bookmark").to_string(),
            sc.next_bookmark.display(),
        ),
        (
            ShortcutTarget::PrevBookmark,
            t!("shortcuts.prev_bookmark").to_string(),
            sc.prev_bookmark.display(),
        ),
        (
            ShortcutTarget::NavBack,
            t!("shortcuts.nav_back").to_string(),
            sc.nav_back.display(),
        ),
        (
            ShortcutTarget::NavForward,
            t!("shortcuts.nav_forward").to_string(),
            sc.nav_forward.display(),
        ),
        (
            ShortcutTarget::ToggleTerminal,
            t!("shortcuts.toggle_terminal").to_string(),
            sc.toggle_terminal.display(),
        ),
        (
            ShortcutTarget::ToggleGit,
            t!("shortcuts.toggle_git").to_string(),
            sc.toggle_git.display(),
        ),
        (
            ShortcutTarget::GitCommit,
            t!("shortcuts.git_commit").to_string(),
            sc.git_commit.display(),
        ),
        (
            ShortcutTarget::GitPush,
            t!("shortcuts.git_push").to_string(),
            sc.git_push.display(),
        ),
        (
            ShortcutTarget::GitPull,
            t!("shortcuts.git_pull").to_string(),
            sc.git_pull.display(),
        ),
        (
            ShortcutTarget::GitBranches,
            t!("shortcuts.git_branches").to_string(),
            sc.git_branches.display(),
        ),
        (
            ShortcutTarget::BookmarkPanel,
            t!("shortcuts.bookmark_panel").to_string(),
            sc.bookmark_panel.display(),
        ),
        (
            ShortcutTarget::GotoFile,
            t!("shortcuts.goto_file").to_string(),
            sc.goto_file.display(),
        ),
        (
            ShortcutTarget::FindInFiles,
            t!("shortcuts.find_in_files").to_string(),
            sc.find_in_files.display(),
        ),
        (
            ShortcutTarget::ReplaceInFiles,
            t!("shortcuts.replace_in_files").to_string(),
            sc.replace_in_files.display(),
        ),
        (
            ShortcutTarget::Complete,
            t!("shortcuts.complete").to_string(),
            sc.complete.display(),
        ),
        (
            ShortcutTarget::ParameterInfo,
            t!("shortcuts.parameter_info").to_string(),
            sc.parameter_info.display(),
        ),
        (
            ShortcutTarget::GotoClass,
            t!("shortcuts.goto_class").to_string(),
            sc.goto_class.display(),
        ),
        (
            ShortcutTarget::GotoSymbol,
            t!("shortcuts.goto_symbol").to_string(),
            sc.goto_symbol.display(),
        ),
        (
            ShortcutTarget::ReopenTab,
            t!("shortcuts.reopen_tab").to_string(),
            sc.reopen_tab.display(),
        ),
        (
            ShortcutTarget::ToggleFold,
            t!("shortcuts.toggle_fold").to_string(),
            sc.toggle_fold.display(),
        ),
        (
            ShortcutTarget::NewFile,
            t!("shortcuts.new_file").to_string(),
            sc.new_file.display(),
        ),
        (
            ShortcutTarget::OpenFile,
            t!("shortcuts.open_file").to_string(),
            sc.open_file.display(),
        ),
        (
            ShortcutTarget::SaveFile,
            t!("shortcuts.save_file").to_string(),
            sc.save_file.display(),
        ),
        (
            ShortcutTarget::SaveAs,
            t!("shortcuts.save_as").to_string(),
            sc.save_as.display(),
        ),
        (
            ShortcutTarget::CloseFile,
            t!("shortcuts.close_file").to_string(),
            sc.close_file.display(),
        ),
        (
            ShortcutTarget::Find,
            t!("shortcuts.find").to_string(),
            sc.find.display(),
        ),
        (
            ShortcutTarget::SelectAll,
            t!("shortcuts.select_all").to_string(),
            sc.select_all.display(),
        ),
        (
            ShortcutTarget::FormatCode,
            t!("shortcuts.format_code").to_string(),
            sc.format_code.display(),
        ),
        (
            ShortcutTarget::GotoLine,
            t!("shortcuts.goto_line").to_string(),
            sc.goto_line.display(),
        ),
        (
            ShortcutTarget::CommandPalette,
            t!("shortcuts.command_palette").to_string(),
            sc.command_palette.display(),
        ),
        (
            ShortcutTarget::ToggleSidebar,
            t!("shortcuts.toggle_sidebar").to_string(),
            sc.toggle_sidebar.display(),
        ),
        (
            ShortcutTarget::Quit,
            t!("shortcuts.quit").to_string(),
            sc.quit.display(),
        ),
    ]
}

/// The groups of the shortcuts of the editor, by theme.
fn editor_groups() -> Vec<(String, Vec<ShortcutTarget>)> {
    use ShortcutTarget as T;
    vec![
        (
            t!("prefs.sc_files").to_string(),
            vec![
                T::NewFile,
                T::OpenFile,
                T::SaveFile,
                T::SaveAs,
                T::CloseFile,
                T::ReopenTab,
                T::Quit,
            ],
        ),
        (
            t!("prefs.sc_edit").to_string(),
            vec![
                T::Undo,
                T::Redo,
                T::SelectAll,
                T::DuplicateLine,
                T::MoveLineUp,
                T::MoveLineDown,
                T::DeleteLine,
                T::ToggleComment,
                T::FormatCode,
                T::RenameSymbol,
            ],
        ),
        (
            t!("prefs.sc_cursors").to_string(),
            vec![
                T::AddNextOccurrence,
                T::SelectAllOccurrences,
                T::AddCursorAbove,
                T::AddCursorBelow,
            ],
        ),
        (
            t!("prefs.sc_code").to_string(),
            vec![T::Complete, T::ParameterInfo, T::ToggleFold],
        ),
        (
            t!("prefs.sc_navigation").to_string(),
            vec![
                T::Find,
                T::FindInFiles,
                T::ReplaceInFiles,
                T::GotoFile,
                T::GotoClass,
                T::GotoSymbol,
                T::GotoLine,
                T::NavBack,
                T::NavForward,
            ],
        ),
        (
            t!("prefs.sc_bookmarks").to_string(),
            vec![
                T::ToggleBookmark,
                T::NextBookmark,
                T::PrevBookmark,
                T::BookmarkPanel,
            ],
        ),
        (
            t!("prefs.sc_view").to_string(),
            vec![T::CommandPalette, T::ToggleSidebar],
        ),
    ]
}

pub fn shortcuts_list(
    dark: bool,
    config: &Config,
    capturing: Option<ShortcutTarget>,
    plugins: &crate::plugins::Plugins,
    filter: &str,
) -> Element<'static, Message> {
    let p = theme::palette(dark);
    let sc = &config.shortcuts;

    let entries = shortcut_entries(sc);

    // Each shortcut is in one group; the ones of the plugins come after, one
    // group per plugin installed.
    let editor_groups = editor_groups();
    let mut groups = editor_groups;
    for (id, targets) in PLUGIN_SHORTCUTS {
        if let Some(plugin) = plugins.get(id) {
            groups.push((plugin.name(), targets.to_vec()));
        }
    }
    // Whatever no group names is kept, at the end: no shortcut is lost.
    let named: Vec<ShortcutTarget> = groups.iter().flat_map(|(_, t)| t.clone()).collect();
    let left: Vec<ShortcutTarget> = entries
        .iter()
        .map(|(target, ..)| *target)
        .filter(|t| !named.contains(t) && shortcut_shown(*t, plugins))
        .collect();
    if !left.is_empty() {
        groups.push((t!("prefs.sc_other").to_string(), left));
    }

    // The search: every word must be in the label or in the keys.
    let words: Vec<String> = filter
        .split(|c: char| c.is_whitespace() || c == '+')
        .filter(|w| !w.is_empty())
        .map(plain)
        .collect();
    let matches = |label: &str, key: &str| {
        let hay = format!("{} {}", plain(label), plain(key));
        words.iter().all(|w| hay.contains(w.as_str()))
    };

    let row_of =
        |target: ShortcutTarget, label: String, key: String| -> Element<'static, Message> {
            let is_capturing = capturing == Some(target);
            let key_label = if is_capturing {
                t!("shortcuts.capturing").to_string()
            } else {
                key
            };
            let key_button = button(
                text(key_label)
                    .size(12)
                    .font(theme::MONO)
                    .color(if is_capturing { p.accent } else { p.text }),
            )
            .padding([4, 10])
            .on_press(Message::StartCaptureShortcut(target))
            .style(
                (theme::Segment {
                    dark,
                    active: is_capturing,
                    radius: theme::R_SM,
                })
                .style(),
            );
            setting_row(label, key_button.into())
        };

    let search = text_input(&t!("prefs.shortcut_search"), filter)
        .on_input(|v| Message::Preferences(PreferencesMessage::ShortcutFilter(v)))
        .padding([8, 12])
        .size(13)
        .style(theme::Field::new(dark).style());

    let mut page = column![search].spacing(22);
    let mut any = false;
    for (title, targets) in groups {
        let rows: Vec<Element<'static, Message>> = targets
            .into_iter()
            .filter_map(|target| {
                let (_, label, key) = entries.iter().find(|(t, ..)| *t == target)?;
                matches(label, key).then(|| row_of(target, label.clone(), key.clone()))
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        any = true;
        page = page.push(
            column![
                text(title)
                    .size(12)
                    .font(theme::UI_SEMIBOLD)
                    .color(theme::muted_strong(dark)),
                card_of_rows(dark, rows),
            ]
            .spacing(8),
        );
    }
    if !any {
        page = page.push(
            text(t!("prefs.shortcut_none").to_string())
                .size(13)
                .color(theme::muted_strong(dark)),
        );
    }
    page.push(
        text(if capturing.is_some() {
            t!("shortcuts.capturing").to_string()
        } else {
            t!("shortcuts.click_to_rebind").to_string()
        })
        .size(11.5)
        .color(theme::muted_strong(dark)),
    )
    .into()
}

/// The shortcuts of the plugins, by plugin: its group shows once it is
/// installed.
const PLUGIN_SHORTCUTS: [(&str, &[ShortcutTarget]); 4] = [
    (
        crate::plugins::migrate::GIT,
        &[
            ShortcutTarget::ToggleGit,
            ShortcutTarget::GitCommit,
            ShortcutTarget::GitPush,
            ShortcutTarget::GitPull,
            ShortcutTarget::GitBranches,
        ],
    ),
    (
        crate::plugins::migrate::TERMINAL,
        &[ShortcutTarget::ToggleTerminal],
    ),
    (crate::plugins::migrate::SPELL, &[ShortcutTarget::SpellFix]),
    (crate::plugins::migrate::TASKS, &[ShortcutTarget::RunTask]),
];

/// A text as the search compares it: lowercase, without accents.
fn plain(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(crate::palette::fold)
        .collect()
}

/// The Lint page, there once the Lint plugin is installed: whether and how
/// problems are shown, then the settings of the plugin itself (its
/// linters, by language).
fn lint_page<'a>(
    config: &Config,
    dark: bool,
    plugins: &'a crate::plugins::Plugins,
) -> Element<'a, Message> {
    use crate::lint::Setting;
    let p = theme::palette(dark);
    let general = card_of_rows(
        dark,
        vec![
            setting_row(
                t!("lint.enabled").to_string(),
                switch(
                    dark,
                    config.lint_enabled,
                    Message::Lint(Setting::Enabled(!config.lint_enabled)),
                ),
            ),
            setting_row(
                t!("lint.highlight").to_string(),
                switch(
                    dark,
                    config.lint_highlight,
                    Message::Lint(Setting::Highlight(!config.lint_highlight)),
                ),
            ),
            setting_row(
                t!("lint.on_open").to_string(),
                switch(
                    dark,
                    config.lint_on_open,
                    Message::Lint(Setting::OnOpen(!config.lint_on_open)),
                ),
            ),
            setting_row(
                t!("lint.trigger").to_string(),
                pick_list(crate::lint::Trigger::ALL, Some(config.lint_trigger), |t| {
                    Message::Lint(Setting::Trigger(t))
                })
                .padding([6, 12])
                .width(260)
                .text_size(13)
                .style(theme::Select { dark }.style())
                .menu_style(theme::Select { dark }.menu_style())
                .into(),
            ),
            setting_row(
                t!("lint.level").to_string(),
                pick_list(crate::lint::Level::ALL, Some(config.lint_level), |l| {
                    Message::Lint(Setting::Level(l))
                })
                .padding([6, 12])
                .width(260)
                .text_size(13)
                .style(theme::Select { dark }.style())
                .menu_style(theme::Select { dark }.menu_style())
                .into(),
            ),
            setting_row(
                t!("lint.fix_level").to_string(),
                pick_list(crate::lint::Level::ALL, Some(config.lint_fix_level), |l| {
                    Message::Lint(Setting::FixLevel(l))
                })
                .padding([6, 12])
                .width(260)
                .text_size(13)
                .style(theme::Select { dark }.style())
                .menu_style(theme::Select { dark }.menu_style())
                .into(),
            ),
            setting_row(
                t!("lint.fix_on_save").to_string(),
                switch(
                    dark,
                    config.lint_fix_on_save,
                    Message::Lint(Setting::FixOnSave(!config.lint_fix_on_save)),
                ),
            ),
        ],
    );
    let colors = card_of_rows(
        dark,
        [
            (crate::lint::Severity::Error, t!("lint.color_error")),
            (crate::lint::Severity::Warning, t!("lint.color_warning")),
            (crate::lint::Severity::Info, t!("lint.color_info")),
            (crate::lint::Severity::Hint, t!("lint.color_hint")),
        ]
        .into_iter()
        .map(|(severity, label)| {
            let shown = crate::lint::color(config, severity);
            let typed = config
                .lint_colors
                .get(crate::lint::color_key(severity))
                .cloned()
                .unwrap_or_default();
            let default = crate::lint::default_color(severity, config.dark_mode);
            let placeholder = format!(
                "#{:02x}{:02x}{:02x}",
                (default.r * 255.0).round() as u8,
                (default.g * 255.0).round() as u8,
                (default.b * 255.0).round() as u8
            );
            setting_row(
                label.to_string(),
                row![
                    pick_list(
                        crate::lint::Marking::ALL,
                        Some(crate::lint::marking(config, severity)),
                        move |m| Message::Lint(Setting::Marking(severity, m)),
                    )
                    .padding([5, 10])
                    .width(210)
                    .text_size(12.5)
                    .style(theme::Select { dark }.style())
                    .menu_style(theme::Select { dark }.menu_style()),
                    container(Space::new().width(18).height(18)).style(move |_: &iced::Theme| {
                        container::Style {
                            background: Some(iced::Background::Color(shown)),
                            border: iced::Border {
                                radius: 0.0.into(),
                                ..iced::Border::default()
                            },
                            ..container::Style::default()
                        }
                    }),
                    iced::widget::text_input(&placeholder, &typed)
                        .on_input(move |t| Message::Lint(Setting::Color(severity, t)))
                        .size(12.5)
                        .padding([5, 8])
                        .width(120)
                        .style(theme::Field::new(dark).style()),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            )
        })
        .collect(),
    );
    let mut page = column![general, colors].spacing(14);
    // The settings of the plugin, drawn as it sends them.
    if let Some(linter) = plugins.linter() {
        if let (Some(panel), Some(body)) = (
            plugins.settings_panel(linter.id()),
            plugin_settings_body(plugins, linter.id(), dark),
        ) {
            page = page.push(
                text(panel.title.get(&rust_i18n::locale()).to_string())
                    .size(14)
                    .font(theme::UI_SEMIBOLD)
                    .color(p.text),
            );
            page = page.push(container(body).padding([4, 4]).width(Length::Fill));
        }
    }
    page.into()
}

/// The settings of a plugin, as it draws them (a note while it is starting).
fn plugin_settings_body<'a>(
    plugins: &'a crate::plugins::Plugins,
    id: &str,
    dark: bool,
) -> Option<Element<'a, Message>> {
    let panel = plugins.settings_panel(id)?;
    let key = (id.to_string(), panel.id.clone());
    Some(match plugins.views.get(&key) {
        Some(node) => crate::plugins::render::view(plugins, &key, node, dark),
        None => text(t!("plugins.panel_loading").to_string())
            .size(12)
            .color(theme::palette(dark).muted)
            .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The settings, as the pages and the search see them.
    fn settings_of<'a>(
        state: &'a PreferencesState,
        config: &'a Config,
        plugins: &'a crate::plugins::Plugins,
    ) -> Vec<groups::Setting<'a>> {
        groups::all(
            state,
            &groups::Context {
                config,
                dark: true,
                plugins,
                spell_choices: &["en_US".to_string(), "fr_FR".to_string()],
            },
        )
    }

    fn in_group(settings: &[groups::Setting], group: Group) -> usize {
        settings.iter().filter(|s| s.group == group).count()
    }

    fn install(plugins: &mut crate::plugins::Plugins, id: &str) {
        let manifest = ardoise_plugin::manifest::Manifest::parse(&format!(
            r#"{{"id": "{id}", "name": "{id}", "version": "1.0.0", "api": 1, "command": ["bin/none"]}}"#
        ))
        .unwrap();
        plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: std::path::PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
    }

    #[test]
    fn every_block_of_settings_has_something_in_it() {
        let config = Config::default();
        let state = PreferencesState::from_config(&config);
        let plugins = crate::plugins::Plugins::default();
        let settings = settings_of(&state, &config, &plugins);
        for group in [
            Group::Display,
            Group::Indentation,
            Group::Typing,
            Group::Completion,
            Group::Files,
            Group::Search,
            Group::Panels,
            Group::Projects,
            Group::Scratch,
            Group::Snippets,
            Group::Trust,
            Group::History,
            Group::Language,
            Group::Appearance,
            Group::Git,
            Group::Spelling,
        ] {
            assert!(in_group(&settings, group) > 0, "{group:?} is empty");
        }
        // High contrast is with the theme, not with the text.
        let appearance: Vec<&str> = settings
            .iter()
            .filter(|s| s.group == Group::Appearance)
            .map(|s| s.label.as_str())
            .collect();
        assert!(appearance.contains(&t!("prefs.high_contrast").as_ref()));
        assert!(settings
            .iter()
            .filter(|s| s.group == Group::Display)
            .all(|s| s.label != t!("prefs.high_contrast")));
    }

    #[test]
    fn what_git_ignores_is_hidden_from_the_page_of_git_when_it_is_installed() {
        let config = Config::default();
        let state = PreferencesState::from_config(&config);
        let mut plugins = crate::plugins::Plugins::default();
        let label = t!("sidebar.hide_ignored").to_string();
        let has = |group: Group, plugins: &crate::plugins::Plugins| {
            settings_of(&state, &config, plugins)
                .iter()
                .any(|s| s.group == group && s.label == label)
        };
        assert!(has(Group::Projects, &plugins) && !has(Group::Git, &plugins));
        install(&mut plugins, crate::plugins::migrate::GIT);
        assert!(has(Group::Git, &plugins) && !has(Group::Projects, &plugins));
        // The tab of Git in Search everywhere is on its page too, spelling
        // has its own.
        let settings = settings_of(&state, &config, &plugins);
        assert!(settings
            .iter()
            .any(|s| s.group == Group::Git && s.label == t!("prefs.everywhere_plugins")));
        assert!(settings
            .iter()
            .any(|s| s.group == Group::Spelling && s.label == t!("prefs.spell_language")));
    }

    #[test]
    fn the_search_of_the_settings_finds_a_label_or_a_block_without_accents() {
        let config = Config::default();
        let state = PreferencesState::from_config(&config);
        let plugins = crate::plugins::Plugins::default();
        let labels = |q: &str| -> Vec<String> {
            groups::matching(settings_of(&state, &config, &plugins), q)
                .into_iter()
                .map(|s| s.label)
                .collect()
        };
        assert!(labels("minimap").contains(&t!("prefs.show_minimap").to_string()));
        assert!(labels("MINIMAP").len() == labels("minimap").len());
        // A word of the block finds all of the block.
        assert!(labels("indentation").len() >= 4);
        // Two words: both must be there.
        assert!(!labels("minimap police")
            .iter()
            .any(|l| l == &t!("prefs.show_minimap")));
        assert!(labels("zzzz").is_empty());
        // Nothing typed: everything.
        assert_eq!(
            labels("").len(),
            settings_of(&state, &config, &plugins).len()
        );
    }

    #[test]
    fn a_block_goes_back_to_its_defaults_and_a_list_has_nothing_to_go_back_to() {
        let defaults = Config::default();
        let mut config = Config {
            font_size: 22.0,
            show_minimap: !defaults.show_minimap,
            word_wrap: !defaults.word_wrap,
            tab_width: 8,
            auto_save: !defaults.auto_save,
            trim_trailing_whitespace: true,
            default_line_ending: "CRLF".into(),
            ..Config::default()
        };
        let mut state = PreferencesState::from_config(&config);
        assert!(state.update(PreferencesMessage::ResetGroup(Group::Display), &mut config));
        assert_eq!(config.font_size, defaults.font_size);
        assert_eq!(config.show_minimap, defaults.show_minimap);
        assert_eq!(config.word_wrap, defaults.word_wrap);
        // The other blocks keep what they have.
        assert_eq!(config.tab_width, 8);
        assert!(state.update(
            PreferencesMessage::ResetGroup(Group::Indentation),
            &mut config
        ));
        assert_eq!(config.tab_width, defaults.tab_width);
        assert!(state.update(PreferencesMessage::ResetGroup(Group::Files), &mut config));
        assert_eq!(config.auto_save, defaults.auto_save);
        assert!(!config.trim_trailing_whitespace && config.default_line_ending.is_empty());
        // A list: nothing changes.
        assert!(!state.update(PreferencesMessage::ResetGroup(Group::Snippets), &mut config));
        assert!(Group::Display.resettable() && !Group::Trust.resettable());
    }

    #[test]
    fn the_lists_the_user_wrote_can_be_edited_from_the_preferences() {
        let mut config = Config::default();
        let mut state = PreferencesState::from_config(&config);
        // A snippet: written, added, removed.
        state.update(PreferencesMessage::SnippetTrigger("fn".into()), &mut config);
        state.update(
            PreferencesMessage::SnippetLanguages("rs, .Py".into()),
            &mut config,
        );
        assert!(
            !state.update(PreferencesMessage::SnippetAdd, &mut config),
            "no body, nothing added"
        );
        state.snippet_body = iced::widget::text_editor::Content::with_text("fn $1() {}");
        assert!(state.update(PreferencesMessage::SnippetAdd, &mut config));
        assert_eq!(config.snippets.len(), 1);
        assert_eq!(config.snippets[0].trigger, "fn");
        assert_eq!(config.snippets[0].languages, vec!["rs", "py"]);
        assert!(state.snippet_trigger.is_empty(), "the form is empty again");
        state.update(PreferencesMessage::SnippetRemove(0), &mut config);
        assert!(config.snippets.is_empty());
        // A trusted folder: no longer trusted.
        config.trusted_folders = vec!["/a".into(), "/b".into()];
        crate::trust::set_all(&config.trusted_folders);
        state.update(PreferencesMessage::UntrustFolder("/a".into()), &mut config);
        assert_eq!(config.trusted_folders, vec!["/b".to_string()]);
        assert!(!crate::trust::is_trusted(std::path::Path::new("/a/x")));
        crate::trust::set_all(&[]);
        // The abbreviation of an action: removed, and the action with it
        // when it was the last.
        config
            .action_abbreviations
            .insert("ToggleSidebar".into(), vec!["sb".into(), "side".into()]);
        state.update(
            PreferencesMessage::AbbreviationRemove("ToggleSidebar".into(), "sb".into()),
            &mut config,
        );
        assert_eq!(config.action_abbreviations["ToggleSidebar"], vec!["side"]);
        state.update(
            PreferencesMessage::AbbreviationRemove("ToggleSidebar".into(), "side".into()),
            &mut config,
        );
        assert!(config.action_abbreviations.is_empty());
        // The histories.
        config.recent_files = vec!["/x".into()];
        config.find_history = vec!["a".into()];
        config.replace_history = vec!["b".into()];
        config.remote_hosts = vec!["me@box".into()];
        for kind in [
            HistoryKind::RecentFiles,
            HistoryKind::Find,
            HistoryKind::Replace,
            HistoryKind::RemoteHosts,
        ] {
            state.update(PreferencesMessage::ClearHistory(kind), &mut config);
        }
        assert!(config.recent_files.is_empty() && config.find_history.is_empty());
        assert!(config.replace_history.is_empty() && config.remote_hosts.is_empty());
    }

    #[test]
    fn a_new_file_is_stored_as_the_settings_say_and_the_font_is_the_one_named() {
        use crate::tabs::{Encoding, LineEnding};
        let mut config = Config::default();
        let mut state = PreferencesState::from_config(&config);
        let format = config.new_file_format();
        assert_eq!(
            (format.encoding, format.ending),
            (Encoding::Utf8, LineEnding::Lf)
        );
        state.update(
            PreferencesMessage::DefaultLineEndingChanged("CRLF".into()),
            &mut config,
        );
        state.update(
            PreferencesMessage::DefaultEncodingChanged(Encoding::Latin1.label().into()),
            &mut config,
        );
        let format = config.new_file_format();
        assert_eq!(
            (format.encoding, format.ending),
            (Encoding::Latin1, LineEnding::Crlf)
        );
        // Back to the defaults: nothing is kept.
        state.update(
            PreferencesMessage::DefaultLineEndingChanged("LF".into()),
            &mut config,
        );
        state.update(
            PreferencesMessage::DefaultEncodingChanged(Encoding::Utf8.label().into()),
            &mut config,
        );
        assert!(config.default_line_ending.is_empty() && config.default_encoding.is_empty());
        // The font: typed, then set; empty is the bundled one.
        state.update(
            PreferencesMessage::FontFamilyTyped(" Fira Code ".into()),
            &mut config,
        );
        assert!(config.font_family.is_empty(), "typing does not apply it");
        state.update(
            PreferencesMessage::FontFamilySet(" Fira Code ".into()),
            &mut config,
        );
        assert_eq!(config.font_family, "Fira Code");
        assert_eq!(
            theme::editor_font(&config.font_family),
            theme::editor_font("Fira Code")
        );
        assert_eq!(theme::editor_font(""), theme::MONO);
        assert_ne!(theme::editor_font("Fira Code"), theme::MONO);
    }

    #[test]
    fn the_spelling_plugin_has_a_page_of_settings_once_installed() {
        let mut plugins = crate::plugins::Plugins::default();
        assert!(plugins.settings_pages().is_empty());
        let manifest = ardoise_plugin::manifest::Manifest::parse(
            r#"{"id": "ardoise.spell", "name": "Spelling", "version": "1.0.0", "api": 1, "command": ["bin/none"]}"#,
        )
        .unwrap();
        plugins.installed.push(crate::plugins::Installed {
            manifest,
            dir: std::path::PathBuf::from("/nowhere"),
            origin: Default::default(),
        });
        let pages = plugins.settings_pages();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].0, "ardoise.spell");
    }

    #[test]
    fn every_shortcut_is_in_exactly_one_group() {
        let entries = shortcut_entries(&Config::default().shortcuts);
        let mut named: Vec<ShortcutTarget> = editor_groups()
            .into_iter()
            .flat_map(|(_, targets)| targets)
            .collect();
        named.extend(
            PLUGIN_SHORTCUTS
                .iter()
                .flat_map(|(_, targets)| targets.to_vec()),
        );
        for (target, ..) in &entries {
            let count = named.iter().filter(|t| *t == target).count();
            assert_eq!(count, 1, "{target:?} is in {count} groups");
        }
        assert_eq!(named.len(), entries.len(), "a group names something else");
    }

    #[test]
    fn the_search_of_the_shortcuts_looks_at_the_label_and_the_keys_without_accents() {
        // What a row matches: the same test as the page applies.
        let hit = |label: &str, key: &str, typed: &str| {
            let words: Vec<String> = typed
                .split(|c: char| c.is_whitespace() || c == '+')
                .filter(|w| !w.is_empty())
                .map(plain)
                .collect();
            let hay = format!("{} {}", plain(label), plain(key));
            words.iter().all(|w| hay.contains(w.as_str()))
        };
        assert!(
            hit("Dupliquer la ligne", "Ctrl+D", "dupli"),
            "a part of the label"
        );
        assert!(
            hit("Rétablir", "Ctrl+Maj+Z", "retablir"),
            "accents do not matter"
        );
        assert!(hit("Rétablir", "Ctrl+Maj+Z", "ctrl+maj+z"), "the keys");
        assert!(hit("Rétablir", "Ctrl+Maj+Z", "maj z"), "words in any place");
        assert!(
            hit("Annuler", "Ctrl+Z", "ANNULER"),
            "capitals do not matter"
        );
        assert!(!hit("Annuler", "Ctrl+Z", "dupli"));
        assert!(!hit("Annuler", "Ctrl+Z", "ctrl+y"));
        assert!(hit("Annuler", "Ctrl+Z", ""), "nothing typed: everything");
    }

    #[test]
    fn the_shortcuts_of_a_plugin_show_once_it_is_installed() {
        let mut plugins = crate::plugins::Plugins::default();
        assert!(!shortcut_shown(ShortcutTarget::GitCommit, &plugins));
        assert!(!shortcut_shown(ShortcutTarget::SpellFix, &plugins));
        assert!(!shortcut_shown(ShortcutTarget::RunTask, &plugins));
        assert!(shortcut_shown(ShortcutTarget::Undo, &plugins));
        for id in ["ardoise.git", "ardoise.spell", "ardoise.tasks"] {
            let manifest = ardoise_plugin::manifest::Manifest::parse(&format!(
                r#"{{"id": "{id}", "name": "{id}", "version": "1.0.0", "api": 1, "command": ["bin/none"]}}"#
            ))
            .unwrap();
            plugins.installed.push(crate::plugins::Installed {
                manifest,
                dir: std::path::PathBuf::from("/nowhere"),
                origin: Default::default(),
            });
        }
        assert!(shortcut_shown(ShortcutTarget::GitCommit, &plugins));
        assert!(shortcut_shown(ShortcutTarget::SpellFix, &plugins));
        assert!(shortcut_shown(ShortcutTarget::RunTask, &plugins));
        assert!(!shortcut_shown(ShortcutTarget::ToggleTerminal, &plugins));
    }

    #[test]
    fn preferences_from_default_config() {
        let config = Config::default();
        let prefs = PreferencesState::from_config(&config);
        assert!(matches!(prefs.page, PrefsPage::Editor));
        assert_eq!(config.font_size, 14.0);
        assert_eq!(config.tab_width, 4);
        assert!(config.use_spaces);
        assert_eq!(config.locale, "en");
        assert!(config.highlight_current_line);
        assert!(!config.show_page_guide);
        assert!(config.show_minimap);
    }

    #[test]
    fn update_font_size() {
        let mut config = Config::default();
        let mut prefs = PreferencesState::from_config(&config);
        assert!(prefs.update(PreferencesMessage::FontSizeChanged(18.0), &mut config));
        assert_eq!(config.font_size, 18.0);
        assert_eq!(config.font_size, 18.0);
    }

    #[test]
    fn update_locale_persists_to_config() {
        let mut config = Config::default();
        let mut prefs = PreferencesState::from_config(&config);
        prefs.update(
            PreferencesMessage::LocaleChanged("fr".to_string()),
            &mut config,
        );
        assert_eq!(config.locale, "fr");
        assert_eq!(config.locale, "fr");
        assert_eq!(rust_i18n::locale().to_string(), "fr");

        prefs.update(
            PreferencesMessage::LocaleChanged("en".to_string()),
            &mut config,
        );
        assert_eq!(config.locale, "en");
        assert_eq!(rust_i18n::locale().to_string(), "en");
    }

    #[test]
    fn update_tab_settings_and_editor_toggles() {
        let mut config = Config::default();
        let mut prefs = PreferencesState::from_config(&config);
        prefs.update(PreferencesMessage::TabWidthChanged(2), &mut config);
        prefs.update(PreferencesMessage::UseSpacesToggled(false), &mut config);
        prefs.update(
            PreferencesMessage::HighlightCurrentLineToggled(false),
            &mut config,
        );
        prefs.update(PreferencesMessage::ShowPageGuideToggled(true), &mut config);
        prefs.update(PreferencesMessage::ShowMinimapToggled(false), &mut config);
        assert_eq!(config.tab_width, 2);
        assert!(!config.use_spaces);
        assert!(!config.highlight_current_line);
        assert!(config.show_page_guide);
        assert!(!config.show_minimap);
        prefs.update(PreferencesMessage::WordWrapToggled(false), &mut config);
        assert!(!config.word_wrap);
    }

    #[test]
    fn page_navigation_does_not_touch_the_config() {
        let mut config = Config::default();
        let mut prefs = PreferencesState::from_config(&config);
        assert!(!prefs.update(
            PreferencesMessage::PageSelected(PrefsPage::Shortcuts),
            &mut config
        ));
        assert_eq!(prefs.page, PrefsPage::Shortcuts);
    }
}
