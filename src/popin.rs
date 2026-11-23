//! The frame every popin shares, and the buttons that are only a picto.
//!
//! A popin is a card on the elevated color with a hairline border and sharp
//! corners, over the dimmed backdrop of [`crate::floating::modal`]: a header
//! (title, a cross), tabs if it has some, a body, a footer with the actions.
//! The pictos are Lucide, drawn from the font the editor already bundles.

use iced::widget::{button, column, container, row, text, tooltip, Space};
use iced::{Alignment, Element, Length};
use rust_i18n::t;

use crate::theme::{self, IconButton, IconTone};

/// The side of a square button that is only a picto.
pub const ICON_BUTTON: f32 = 28.0;
/// The same, in the footer of a popin.
pub const ICON_BUTTON_FOOTER: f32 = 32.0;
/// The size of a picto.
pub const ICON_SIZE: f32 = 16.0;
/// A form: tabs, fields.
pub const WIDTH_FORM: f32 = 600.0;
/// A confirmation.
pub const WIDTH_CONFIRM: f32 = 420.0;
/// A card with a little text.
pub const WIDTH_CARD: f32 = 360.0;

/// The most a body is high before it scrolls.
pub const MAX_BODY: f32 = 520.0;

/// The glyphs of the Lucide icons the plugins and the editor draw by name.
const LUCIDE: &[(&str, u32)] = &[
    ("archive", 0xe045),
    ("archive-restore", 0xe2cd),
    ("arrow-left-right", 0xe24a),
    ("book-open", 0xe063),
    ("check", 0xe070),
    ("chevrons-up-down", 0xe211),
    ("circle-help", 0xe082),
    ("code-xml", 0xe206),
    ("columns-2", 0xe09c),
    ("copy", 0xe0a2),
    ("database", 0xe0b1),
    ("download", 0xe0b6),
    ("eye", 0xe0be),
    ("eye-off", 0xe0bf),
    ("file", 0xe0c4),
    ("file-input", 0xe0c9),
    ("file-plus", 0xe0cd),
    ("folder-open", 0xe247),
    ("folder-plus", 0xe0dd),
    ("git-compare", 0xe35d),
    ("hard-drive-download", 0xe4ea),
    ("hash", 0xe0f3),
    ("heart-pulse", 0xe372),
    ("import", 0xe22f),
    ("info", 0xe0ff),
    ("key-round", 0xe4a8),
    ("languages", 0xe104),
    ("link", 0xe108),
    ("list-tree", 0xe40d),
    ("log-out", 0xe112),
    ("panel-bottom", 0xe431),
    ("pencil", 0xe1f9),
    ("play", 0xe140),
    ("plus", 0xe141),
    ("puzzle", 0xe29c),
    ("refresh-cw", 0xe149),
    ("rotate-ccw", 0xe14c),
    ("save", 0xe151),
    ("search", 0xe155),
    ("shield-check", 0xe1ff),
    ("sliders-horizontal", 0xe29a),
    ("sparkles", 0xe417),
    ("spell-check", 0xe49f),
    ("square", 0xe16b),
    ("table-2", 0xe2f9),
    ("terminal", 0xe185),
    ("text-cursor-input", 0xe265),
    ("trash-2", 0xe18e),
    ("triangle-alert", 0xe193),
    ("type", 0xe198),
    ("x", 0xe1b2),
    ("zap", 0xe1b4),
];

/// The glyph of a Lucide icon known by its name.
pub fn lucide(name: &str) -> Option<char> {
    LUCIDE
        .iter()
        .find(|(n, _)| *n == name)
        .and_then(|(_, code)| char::from_u32(*code))
}

/// A picto as text: a Lucide icon by name, else the one character it is (a
/// glyph of the Nerd Fonts, as for the icon of a list row). `None` for a name
/// that is neither.
pub fn glyph<'a>(icon: &str, size: f32) -> Option<iced::widget::Text<'a>> {
    if let Some(c) = lucide(icon) {
        return Some(
            text(c.to_string())
                .size(size)
                .font(iced_fonts::LUCIDE_FONT)
                .line_height(1.0),
        );
    }
    let mut chars = icon.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(
            text(c.to_string())
                .size(size)
                .font(iced_fonts::NERD_FONT)
                .line_height(1.0),
        ),
        _ => None,
    }
}

/// A square button that is only a picto, with its tooltip (which is also its
/// accessible name). `on_press` none: disabled.
pub fn icon_button<'a, M: Clone + 'a>(
    dark: bool,
    icon: &str,
    tip: &str,
    on_press: Option<M>,
    tone: IconTone,
) -> Option<Element<'a, M>> {
    let picto = glyph(icon, ICON_SIZE)?;
    let side = if tone == IconTone::Outline {
        ICON_BUTTON_FOOTER
    } else {
        ICON_BUTTON
    };
    let b = button(
        container(picto)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding(0)
    .width(side)
    .height(side)
    .on_press_maybe(on_press)
    .style(IconButton { dark, tone }.style());
    Some(
        tooltip(
            b,
            container(text(tip.to_string()).size(12))
                .padding([4, 8])
                .style(theme::tooltip(dark)),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .delay(std::time::Duration::from_millis(400))
        .into(),
    )
}

/// The cross that closes a popin.
pub fn close_button<'a, M: Clone + 'a>(dark: bool, on_close: M) -> Element<'a, M> {
    icon_button(
        dark,
        "x",
        &t!("popin.close"),
        Some(on_close),
        IconTone::Muted,
    )
    .expect("x is a Lucide icon")
}

const FOOTER: f32 = 60.0;
const MARGIN: f32 = 20.0;
const TILE: f32 = 44.0;

/// The header of a popin: a picto in a tile, a title, a line under it.
#[derive(Debug, Clone)]
pub struct Head {
    pub icon: String,
    pub title: String,
    pub subtitle: Option<String>,
    /// The tile is red: something is lost.
    pub danger: bool,
}

impl Head {
    pub fn new(icon: impl Into<String>, title: impl Into<String>) -> Self {
        Head {
            icon: icon.into(),
            title: title.into(),
            subtitle: None,
            danger: false,
        }
    }

    pub fn sub(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }
}

fn hairline<'a, M: 'a>(dark: bool) -> Element<'a, M> {
    container(Space::new().height(1))
        .width(Length::Fill)
        .style(theme::popin_rule(dark))
        .into()
}

/// The actions of a popin: at the left what is secondary, at the right the
/// others.
pub type Actions<'a, M> = (Vec<Element<'a, M>>, Vec<Element<'a, M>>);

/// What goes in a popin besides its body.
pub struct Frame<'a, M> {
    /// Its header; none: the body only (a card with no title).
    pub head: Option<Head>,
    /// The cross, when it has a header: what closes it.
    pub on_close: Option<M>,
    /// Between the header and the body.
    pub tabs: Option<Element<'a, M>>,
    /// The footer: secondary actions at the left, the others at the right.
    pub footer: Option<Actions<'a, M>>,
    pub width: f32,
}

impl<'a, M> Frame<'a, M> {
    pub fn new(width: f32) -> Self {
        Frame {
            head: None,
            on_close: None,
            tabs: None,
            footer: None,
            width,
        }
    }
}

/// The header: tile, title and subtitle, cross.
pub(crate) fn header<'a, M: Clone + 'a>(
    dark: bool,
    head: Head,
    on_close: Option<M>,
) -> Element<'a, M> {
    let p = theme::palette(dark);
    let tile = container(glyph(&head.icon, 22.0).unwrap_or_else(|| text("")))
        .width(TILE)
        .height(TILE)
        .center_x(TILE)
        .center_y(TILE)
        .style(theme::popin_tile(dark, head.danger));
    let mut words = column![text(head.title)
        .size(15)
        .font(theme::UI_SEMIBOLD)
        .color(p.text)
        .shaping(iced::widget::text::Shaping::Advanced)]
    .spacing(2)
    .width(Length::Fill);
    if let Some(sub) = head.subtitle {
        words = words.push(
            text(sub)
                .size(12)
                .color(theme::muted_strong(dark))
                .shaping(iced::widget::text::Shaping::Advanced),
        );
    }
    let mut line = row![tile, words].spacing(14).align_y(Alignment::Center);
    if let Some(close) = on_close {
        line = line.push(
            container(close_button(dark, close))
                .align_y(Alignment::Start)
                .height(TILE),
        );
    }
    container(line).padding(MARGIN).width(Length::Fill).into()
}

/// `body` in the frame of a popin.
pub fn frame<'a, M: Clone + 'a>(
    dark: bool,
    parts: Frame<'a, M>,
    body: Element<'a, M>,
) -> Element<'a, M> {
    let mut card = column![];
    let headed = parts.head.is_some();
    let has_tabs = parts.tabs.is_some();
    if let Some(head) = parts.head {
        card = card.push(header(dark, head, parts.on_close));
    }
    if let Some(tabs) = parts.tabs {
        card = card
            .push(container(tabs).padding([0, 12]))
            .push(hairline(dark));
    }
    // The body sits under the header with its own margin; without a header
    // it has the room all around.
    let top = if headed { 0.0 } else { MARGIN };
    card = card.push(
        container(body)
            .padding(iced::Padding {
                top: if has_tabs { MARGIN } else { top },
                right: MARGIN,
                bottom: MARGIN,
                left: MARGIN,
            })
            .width(Length::Fill),
    );
    if let Some((secondary, actions)) = parts.footer {
        let bar = row(secondary)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(Space::new().width(Length::Fill))
            .push(row(actions).spacing(8).align_y(Alignment::Center));
        card = card.push(hairline(dark)).push(
            container(bar)
                .height(FOOTER)
                .padding([0.0, 16.0])
                .align_y(Alignment::Center)
                .style(theme::popin_footer(dark)),
        );
    }
    container(card)
        .width(parts.width)
        .style(theme::popin_card(dark))
        .into()
}

/// A popin with a header, a body and, if it has some, actions.
pub fn card<'a, M: Clone + 'a>(
    dark: bool,
    width: f32,
    head: Head,
    on_close: Option<M>,
    body: Element<'a, M>,
    footer: Option<Actions<'a, M>>,
) -> Element<'a, M> {
    let mut parts = Frame::new(width);
    parts.head = Some(head);
    parts.on_close = on_close;
    parts.footer = footer;
    frame(dark, parts, body)
}

/// A button of the footer of a popin: its text, never a picto.
pub fn action<'a, M: Clone + 'a>(
    dark: bool,
    label: impl Into<String>,
    on_press: Option<M>,
    kind: Action,
) -> Element<'a, M> {
    let b = button(
        text(label.into())
            .size(13)
            .shaping(iced::widget::text::Shaping::Advanced),
    )
    .padding([7, 14])
    .on_press_maybe(on_press);
    // The buttons of a popin have the corners of its controls.
    theme::in_popin(|| match kind {
        // A hairline of the popin's own, which shows on its color.
        Action::Cancel => b.style(
            IconButton {
                dark,
                tone: IconTone::Outline,
            }
            .style(),
        ),
        Action::Primary => b.style(theme::Primary { dark }.style()),
        Action::Danger => b.style(theme::Danger { dark }.style()),
    })
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Cancel,
    Primary,
    Danger,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_glyph_is_in_the_lucide_font() {
        let face = ttf_parser::Face::parse(iced_fonts::LUCIDE_FONT_BYTES, 0)
            .expect("the Lucide font parses");
        for (name, _) in LUCIDE {
            let c = lucide(name).expect(name);
            assert!(face.glyph_index(c).is_some(), "{name} is not in the font");
            assert_eq!(face.glyph_name(face.glyph_index(c).unwrap()), Some(*name));
        }
    }

    #[test]
    fn a_name_is_a_lucide_icon_a_character_is_itself_and_the_rest_is_nothing() {
        assert!(glyph("plus", 16.0).is_some());
        assert!(glyph("\u{f1c0}", 16.0).is_some());
        assert!(glyph("not-an-icon", 16.0).is_none());
        assert!(glyph("", 16.0).is_none());
        assert_eq!(lucide("plus"), Some('\u{e141}'));
        assert_eq!(lucide("nope"), None);
    }
}
