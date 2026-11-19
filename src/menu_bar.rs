use iced::{
    widget::{button, column, container, mouse_area, row, text, Space},
    Alignment, Element, Length,
};

use crate::app::{Message, TopMenu};
use crate::config::Config;
use crate::menus::{self, Entry, MenuState, SubMenu};
use crate::theme;

/// The title bar: the menus on the left and the Dark/Light switch on the
/// right. No layout side effects; the dropdown is drawn separately.
pub fn view(config: &Config, open: Option<TopMenu>) -> Element<'static, Message> {
    let dark = config.dark_mode;

    // Each button sits in its own `container` carrying a stable `Id`, so its
    // dropdown can be anchored to the button's actual rendered position
    // (queried via `container::visible_bounds`) instead of a hardcoded pixel
    // offset that only happened to fit one language's label widths.
    let top_button = |label: String, menu: TopMenu| {
        // Hovering a title while another menu is open switches to it, as in
        // any standard menu bar.
        mouse_area(
            container(
                button(text(label).size(13))
                    .padding([5, 9])
                    .on_press(Message::ToggleMenu(menu))
                    .style(
                        (theme::MenuTitle {
                            dark,
                            open: open == Some(menu),
                        })
                        .style(),
                    ),
            )
            .id(menu_id(menu)),
        )
        .on_enter(Message::HoverMenu(menu))
    };

    // Sombre | Clair, a pill: the active side wears the accent wash.
    let side = |label: String, is_dark: bool| {
        button(text(label).size(12))
            .padding([3, 10])
            .on_press(Message::ThemeChanged(is_dark))
            .style(
                (theme::Segment {
                    dark,
                    active: dark == is_dark,
                    radius: theme::R_PILL,
                })
                .style(),
            )
    };
    let p = theme::palette(dark);
    let theme_switch = container(
        row![
            side(t!("toolbar.dark").to_string(), true),
            side(t!("toolbar.light").to_string(), false),
        ]
        .spacing(2),
    )
    .padding(2)
    .style(move |_: &iced::Theme| container::Style {
        text_color: Some(p.text),
        background: Some(iced::Background::Color(p.elevated)),
        border: iced::Border {
            color: p.border,
            width: 1.0,
            radius: theme::R_PILL.into(),
        },
        shadow: iced::Shadow::default(),
        ..container::Style::default()
    });

    // On macOS the menus live in the system menu bar (`native_menu`).
    let titles = if cfg!(target_os = "macos") {
        row![]
    } else {
        row![
            top_button(t!("menu.file").to_string(), TopMenu::File),
            top_button(t!("menu.edit").to_string(), TopMenu::Edit),
            top_button(t!("menu.view").to_string(), TopMenu::View),
            top_button(t!("menu.help").to_string(), TopMenu::Help),
        ]
        .spacing(2)
    };
    let bar = row![titles, Space::new().width(Length::Fill), theme_switch]
        .spacing(2)
        .padding([0, 14])
        .height(42)
        .align_y(Alignment::Center);

    container(bar)
        .width(Length::Fill)
        .style(theme::bar(dark))
        .into()
}

/// What the bookmark entries need to know about the active file.
#[derive(Debug, Clone, Copy, Default)]
pub struct BookmarkState {
    /// The cursor's line is bookmarked.
    pub on_line: bool,
    /// The file has at least one bookmark.
    pub any: bool,
}

/// The bookmark entries shared by the Edit menu and the context menu: add or
/// remove the cursor line's bookmark, and remove them all.
pub fn bookmark_items(
    state: BookmarkState,
    dark: bool,
    toggle_hint: Option<String>,
) -> Vec<Element<'static, Message>> {
    let toggle_label = if state.on_line {
        t!("menu.bookmark_remove")
    } else {
        t!("menu.bookmark_add")
    };
    let clear: Element<'static, Message> = if state.any {
        item(
            t!("menu.bookmark_clear").to_string(),
            None,
            dark,
            Message::ClearBookmarks,
        )
    } else {
        disabled_item(t!("menu.bookmark_clear").to_string(), dark)
    };
    vec![
        item(
            toggle_label.to_string(),
            toggle_hint,
            dark,
            Message::ToggleBookmark,
        ),
        clear,
    ]
}

/// The height of a menu item and of a separator in a card, to put a submenu
/// level with the entry that opens it.
const ITEM_HEIGHT: f32 = 31.9;
const SEPARATOR_HEIGHT: f32 = 8.0;

/// An entry of a menu, drawn.
fn drawn(entry: &Entry, dark: bool) -> Element<'static, Message> {
    match entry {
        Entry::Item {
            label,
            shortcut,
            enabled: true,
            message,
        } => item(label.clone(), shortcut.clone(), dark, message.clone()),
        Entry::Item { label, .. } => disabled_item(label.clone(), dark),
        Entry::Separator => separator(dark),
        Entry::Sub {
            id,
            label,
            enabled: true,
            ..
        } => mouse_area(item(
            label.clone(),
            Some("›".to_string()),
            dark,
            Message::SetSubmenu(Some(*id)),
        ))
        .on_enter(Message::SetSubmenu(Some(*id)))
        .into(),
        Entry::Sub { label, .. } => disabled_item(label.clone(), dark),
    }
}

/// The floating dropdown panel for the active menu, with the submenu that is
/// open beside it.
pub fn dropdown_view(
    menu: TopMenu,
    state: &MenuState,
    open: Option<SubMenu>,
) -> Element<'static, Message> {
    let dark = state.config.dark_mode;
    let entries = menus::entries(menu, state, false);
    // Where the open submenu goes: level with its entry.
    let mut at = 0.0;
    let mut flyout: Option<(f32, SubMenu, &Vec<Entry>)> = None;
    let mut rows: Vec<Element<'static, Message>> = Vec::new();
    for entry in &entries {
        if let Entry::Sub {
            id,
            enabled: true,
            entries: inside,
            ..
        } = entry
        {
            if open == Some(*id) {
                flyout = Some((at, *id, inside));
            }
        }
        at += match entry {
            Entry::Separator => SEPARATOR_HEIGHT,
            _ => ITEM_HEIGHT,
        };
        let drawn = drawn(entry, dark);
        rows.push(match entry {
            // Pointing at any other entry folds the submenu away.
            Entry::Sub { enabled: true, .. } => drawn,
            _ => mouse_area(drawn).on_enter(Message::SetSubmenu(None)).into(),
        });
    }
    let card = container(column(rows).spacing(1).padding(6).width(260)).style(theme::card(dark));
    let Some((top, id, inside)) = flyout else {
        return card.into();
    };
    let width = if id == SubMenu::Recents { 300 } else { 260 };
    let beside = container(
        column(inside.iter().map(|entry| drawn(entry, dark)))
            .spacing(1)
            .padding(6)
            .width(width),
    )
    .style(theme::card(dark));
    row![card, column![Space::new().height(top), beside]]
        .spacing(4)
        .into()
}

/// The stable `Id` of a top menu button's wrapping container, so its
/// dropdown can be anchored to wherever that button actually renders —
/// independent of label length, font, or DPI.
pub fn menu_id(menu: TopMenu) -> iced::widget::Id {
    match menu {
        TopMenu::File => iced::widget::Id::new("menu-file"),
        TopMenu::Edit => iced::widget::Id::new("menu-edit"),
        TopMenu::View => iced::widget::Id::new("menu-view"),
        TopMenu::Help => iced::widget::Id::new("menu-help"),
    }
}

/// A clickable menu item with an optional shortcut hint on the right.
pub fn item(
    label: String,
    shortcut: Option<String>,
    dark: bool,
    message: Message,
) -> Element<'static, Message> {
    let content: Element<'static, Message> = if let Some(sc) = shortcut {
        row![
            text(label)
                .size(13)
                .shaping(iced::widget::text::Shaping::Advanced),
            Space::new().width(Length::Fill),
            text(sc).size(11.5).color(theme::muted_text(dark)),
        ]
        .align_y(Alignment::Center)
        .into()
    } else {
        text(label)
            .size(13)
            .shaping(iced::widget::text::Shaping::Advanced)
            .into()
    };

    button(content)
        .padding([7, 10])
        .width(Length::Fill)
        .on_press(message)
        .style(
            (theme::MenuItem {
                dark,
                radius: theme::R_SM,
                highlighted: false,
            })
            .style(),
        )
        .into()
}

pub fn disabled_item(label: String, dark: bool) -> Element<'static, Message> {
    container(text(label).size(13).color(theme::muted_text(dark)))
        .padding([7, 10])
        .width(Length::Fill)
        .into()
}

pub fn separator(dark: bool) -> Element<'static, Message> {
    container(
        container(Space::new().height(1))
            .width(Length::Fill)
            .style(theme::rule(dark)),
    )
    .padding([3, 4])
    .width(Length::Fill)
    .into()
}
