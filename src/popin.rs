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

const HEADER: f32 = 48.0;
const FOOTER: f32 = 56.0;
const MARGIN: f32 = 20.0;

/// The glyphs of the Lucide icons the plugins and the editor draw by name.
const LUCIDE: &[(&str, u32)] = &[
    ("check", 0xe070),
    ("chevrons-up-down", 0xe211),
    ("copy", 0xe0a2),
    ("eye", 0xe0be),
    ("eye-off", 0xe0bf),
    ("pencil", 0xe1f9),
    ("play", 0xe140),
    ("plus", 0xe141),
    ("refresh-cw", 0xe149),
    ("square", 0xe16b),
    ("terminal", 0xe185),
    ("trash-2", 0xe18e),
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
    /// Its title; none: no header (a confirmation).
    pub title: Option<String>,
    /// The cross, when it has a header: what closes it.
    pub on_close: Option<M>,
    /// Between the header and the body.
    pub tabs: Option<Element<'a, M>>,
    /// The actions: at the left what is secondary (pictos), at the right
    /// Cancel then the main action.
    pub footer: Option<Actions<'a, M>>,
    pub width: f32,
}

impl<'a, M> Frame<'a, M> {
    pub fn new(width: f32) -> Self {
        Frame {
            title: None,
            on_close: None,
            tabs: None,
            footer: None,
            width,
        }
    }
}

/// `body` in the frame of a popin.
pub fn frame<'a, M: Clone + 'a>(
    dark: bool,
    parts: Frame<'a, M>,
    body: Element<'a, M>,
) -> Element<'a, M> {
    let p = theme::palette(dark);
    let mut card = column![];
    if let Some(title) = parts.title {
        let mut header = row![text(title)
            .size(15)
            .font(theme::UI_SEMIBOLD)
            .color(p.text)
            .shaping(iced::widget::text::Shaping::Advanced)]
        .align_y(Alignment::Center)
        .push(Space::new().width(Length::Fill));
        if let Some(close) = parts.on_close {
            header = header.push(close_button(dark, close));
        }
        card = card
            .push(
                container(header)
                    .height(HEADER)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 10.0,
                        bottom: 0.0,
                        left: MARGIN,
                    })
                    .align_y(Alignment::Center),
            )
            .push(hairline(dark));
    }
    if let Some(tabs) = parts.tabs {
        card = card.push(tabs).push(hairline(dark));
    }
    card = card.push(container(body).padding(MARGIN).width(Length::Fill));
    if let Some((secondary, actions)) = parts.footer {
        let bar = row(secondary)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(Space::new().width(Length::Fill))
            .push(row(actions).spacing(8).align_y(Alignment::Center));
        card = card.push(hairline(dark)).push(
            container(bar)
                .height(FOOTER)
                .padding([0.0, MARGIN])
                .align_y(Alignment::Center),
        );
    }
    container(card)
        .width(parts.width)
        .style(theme::card(dark))
        .into()
}

/// A popin with a title, a cross, a body and, if it has some, actions: Cancel
/// (what `cancel` says) then the main one.
pub fn card<'a, M: Clone + 'a>(
    dark: bool,
    width: f32,
    title: impl Into<String>,
    on_close: Option<M>,
    body: Element<'a, M>,
    footer: Option<Actions<'a, M>>,
) -> Element<'a, M> {
    let mut parts = Frame::new(width);
    parts.title = Some(title.into());
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
    match kind {
        Action::Cancel => b.style(theme::Outline { dark }.style()),
        Action::Primary => b.style(theme::Primary { dark }.style()),
        Action::Danger => b.style(theme::Danger { dark }.style()),
    }
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
