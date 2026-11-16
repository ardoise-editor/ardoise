//! What a panel of a plugin shows, drawn with the editor's own widgets:
//! the tree of [`Node`]s the plugin sent, its controls sending back
//! [`PluginMsg::Ui`] (and [`PluginMsg::Input`] for the fields).

use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};
use serde_json::{json, Value};
use simpleedit_plugin::ui::{ButtonStyle, Item, Node, TextStyle, TreeItem};

use super::{PanelKey, PluginMsg, Plugins};
use crate::app::Message;
use crate::theme as se_theme;

/// How deep a tree of nodes is drawn: deeper is cut (a plugin's bug must not
/// overflow the stack of the editor).
const MAX_DEPTH: usize = 64;

pub fn view<'a>(
    plugins: &'a Plugins,
    key: &PanelKey,
    node: &'a Node,
    dark: bool,
) -> Element<'a, Message> {
    draw(plugins, key, node, dark, 0)
}

fn ui(key: &PanelKey, action: &str, value: Option<Value>) -> Message {
    Message::Plugin(PluginMsg::Ui(
        key.0.clone(),
        key.1.clone(),
        action.to_string(),
        value,
    ))
}

fn draw<'a>(
    plugins: &'a Plugins,
    key: &PanelKey,
    node: &'a Node,
    dark: bool,
    depth: usize,
) -> Element<'a, Message> {
    let p = se_theme::palette(dark);
    if depth > MAX_DEPTH {
        return Space::new().into();
    }
    match node {
        Node::Column {
            children,
            spacing,
            padding,
        } => column(
            children
                .iter()
                .map(|c| draw(plugins, key, c, dark, depth + 1)),
        )
        .spacing(spacing.unwrap_or(6.0))
        .padding(padding.unwrap_or(0.0))
        .width(Length::Fill)
        .into(),
        Node::Row { children, spacing } => row(children
            .iter()
            .map(|c| draw(plugins, key, c, dark, depth + 1)))
        .spacing(spacing.unwrap_or(6.0))
        .align_y(Alignment::Center)
        .into(),
        Node::Text { text: t, style } => {
            let label = text(t.as_str()).shaping(iced::widget::text::Shaping::Advanced);
            match style {
                TextStyle::Normal => label.size(13).color(p.text),
                TextStyle::Muted => label.size(12).color(p.muted),
                TextStyle::Error => label.size(12.5).color(se_theme::error_color(dark)),
                TextStyle::Bold => label.size(13).font(se_theme::UI_SEMIBOLD).color(p.text),
                TextStyle::Heading => label.size(15).font(se_theme::UI_BOLD).color(p.text),
                TextStyle::Mono => label.size(12.5).font(se_theme::MONO).color(p.text),
            }
            .into()
        }
        Node::Button {
            label,
            action,
            style,
            enabled,
        } => {
            let style = *style;
            let b = button(text(label.as_str()).size(12.5))
                .padding([5, 12])
                .on_press_maybe(enabled.then(|| ui(key, action, None)))
                .style(move |theme: &iced::Theme, status| match style {
                    ButtonStyle::Primary => (se_theme::Primary { dark }).style()(theme, status),
                    ButtonStyle::Outline => (se_theme::Outline { dark }).style()(theme, status),
                    ButtonStyle::Ghost => (se_theme::GhostButton {
                        dark,
                        active: false,
                    })
                    .style()(theme, status),
                    ButtonStyle::Danger => {
                        let mut look = (se_theme::Outline { dark }).style()(theme, status);
                        look.text_color = se_theme::error_color(dark);
                        look
                    }
                });
            b.into()
        }
        Node::Input {
            id,
            value,
            placeholder,
            submit,
            secret,
        } => {
            let shown = plugins.input_text(key, id, value);
            let (k, field) = (key.clone(), id.clone());
            let mut input = text_input(placeholder, shown)
                .on_input(move |v| {
                    Message::Plugin(PluginMsg::Input(k.0.clone(), k.1.clone(), field.clone(), v))
                })
                .secure(*secret)
                .padding([5, 8])
                .size(13)
                .style(se_theme::Field::new(dark).style());
            if let Some(action) = submit {
                input = input.on_submit(ui(key, action, Some(json!(shown))));
            }
            input.into()
        }
        Node::Checkbox { id, label, checked } => {
            let (k, id) = (key.clone(), id.clone());
            checkbox(*checked)
                .label(label.as_str())
                .size(15)
                .text_size(13)
                .on_toggle(move |on| ui(&k, &id, Some(json!(on))))
                .into()
        }
        Node::Select {
            id,
            options,
            selected,
            placeholder,
        } => {
            let (k, id) = (key.clone(), id.clone());
            pick_list(options.as_slice(), selected.clone(), move |v: String| {
                ui(&k, &id, Some(json!(v)))
            })
            .placeholder(placeholder.as_str())
            .padding([5, 10])
            .text_size(13)
            .style(se_theme::Select { dark }.style())
            .menu_style(se_theme::Select { dark }.menu_style())
            .into()
        }
        Node::List {
            id,
            items,
            selected,
        } => column(items.iter().map(|item| {
            let on = selected.as_deref() == Some(item.id.as_str());
            list_row(item, on, dark, ui(key, id, Some(json!(item.id))))
        }))
        .width(Length::Fill)
        .into(),
        Node::Tree { id, items } => {
            let mut rows = Vec::new();
            flatten(items, 0, &mut rows);
            column(
                rows.into_iter()
                    .map(|(item, level)| tree_row(key, id, item, level, dark)),
            )
            .width(Length::Fill)
            .into()
        }
        Node::Separator => container(Space::new().height(1))
            .width(Length::Fill)
            .style(se_theme::rule(dark))
            .into(),
        Node::Space { height } => Space::new().height(height.clamp(0.0, 400.0)).into(),
        Node::Scroll { child, bottom } => {
            let s = scrollable(draw(plugins, key, child, dark, depth + 1))
                .style(se_theme::Scrollbar { dark }.style())
                .height(Length::Fill)
                .width(Length::Fill);
            if *bottom {
                s.anchor_bottom().into()
            } else {
                s.into()
            }
        }
    }
}

/// One character before a label, in the font of the icons.
fn glyph(icon: &Option<String>, color: iced::Color) -> Element<'static, Message> {
    match icon {
        Some(icon) => text(icon.clone())
            .font(iced_fonts::NERD_FONT)
            .size(13)
            .color(color)
            .width(16)
            .into(),
        None => Space::new().width(0).into(),
    }
}

fn list_row<'a>(item: &'a Item, selected: bool, dark: bool, on: Message) -> Element<'a, Message> {
    let p = se_theme::palette(dark);
    let mut line = row![
        glyph(&item.icon, p.muted),
        text(item.label.as_str()).size(13)
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Some(detail) = &item.detail {
        line = line.push(text(detail.as_str()).size(11.5).color(p.muted));
    }
    button(line)
        .padding([4, 8])
        .width(Length::Fill)
        .on_press(on)
        .style((se_theme::SidebarRow { dark, selected }).style())
        .into()
}

/// The rows of a tree that show: the children of the open ones, with
/// their depth.
fn flatten<'a>(items: &'a [TreeItem], level: usize, out: &mut Vec<(&'a TreeItem, usize)>) {
    for item in items {
        out.push((item, level));
        if item.expanded && level < MAX_DEPTH {
            flatten(&item.children, level + 1, out);
        }
    }
}

fn tree_row<'a>(
    key: &PanelKey,
    tree: &str,
    item: &'a TreeItem,
    level: usize,
    dark: bool,
) -> Element<'a, Message> {
    let p = se_theme::palette(dark);
    let opens = item.expandable || !item.children.is_empty();
    let chevron: Element<Message> = if opens {
        button(
            text(if item.expanded {
                "\u{eab4}"
            } else {
                "\u{eab6}"
            })
            .font(iced_fonts::NERD_FONT)
            .size(11)
            .color(p.muted),
        )
        .padding([2, 2])
        .on_press(ui(
            key,
            tree,
            Some(json!({"item": item.id, "kind": "toggle"})),
        ))
        .style(
            (se_theme::GhostButton {
                dark,
                active: false,
            })
            .style(),
        )
        .into()
    } else {
        Space::new().width(15).into()
    };
    let mut label = row![
        glyph(&item.icon, p.muted),
        text(item.label.as_str()).size(13)
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Some(detail) = &item.detail {
        label = label.push(text(detail.as_str()).size(11.5).color(p.muted));
    }
    row![
        Space::new().width(level.min(MAX_DEPTH) as f32 * 14.0),
        chevron,
        button(label)
            .padding([3, 6])
            .width(Length::Fill)
            .on_press(ui(
                key,
                tree,
                Some(json!({"item": item.id, "kind": "click"}))
            ))
            .style(
                (se_theme::GhostButton {
                    dark,
                    active: false,
                })
                .style()
            ),
    ]
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use simpleedit_plugin::ui::tree_item;

    #[test]
    fn only_the_open_rows_of_a_tree_show() {
        let items = vec![
            tree_item("a", "A")
                .expanded(true)
                .children([tree_item("a/1", "1"), tree_item("a/2", "2")]),
            tree_item("b", "B").children([tree_item("b/1", "1")]),
        ];
        let mut rows = Vec::new();
        flatten(&items, 0, &mut rows);
        let shown: Vec<(&str, usize)> = rows.iter().map(|(i, l)| (i.id.as_str(), *l)).collect();
        assert_eq!(shown, vec![("a", 0), ("a/1", 1), ("a/2", 1), ("b", 0)]);
    }
}
