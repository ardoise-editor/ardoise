//! The grids of the panels of plugins (the rows of a table): drawn with
//! only the rows and columns on screen, picked and typed in with the mouse
//! and the keyboard. What the user does goes to the plugin as a
//! [`GridEvent`]; what is only seen (scrolling, a column being dragged
//! wider) stays here.

use std::collections::HashMap;

use ardoise_plugin::ui::{Grid, GridEvent, GridSelection, Node, RowState};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

use super::{PanelKey, PluginMsg};
use crate::app::Message;
use crate::theme as se_theme;

pub const ROW_HEIGHT: f32 = 26.0;
/// The column of the row numbers.
const GUTTER: f32 = 56.0;
/// The edge of a column title, to drag.
const HANDLE: f32 = 4.0;
const DEFAULT_WIDTH: f32 = 160.0;
const MIN_WIDTH: f32 = 40.0;

/// What is only seen of a grid: where it is scrolled, a column being made
/// wider.
#[derive(Debug, Clone, Default)]
pub struct GridView {
    /// Vertical: offset and height shown; across: offset and width shown.
    pub scroll: (f32, f32),
    pub across: (f32, f32),
    /// A column being dragged: which, its width when it was grabbed, where
    /// the pointer was then (once it moved).
    pub resizing: Option<(usize, f32, Option<f32>)>,
    /// Widths dragged and not yet told to the plugin.
    pub widths: HashMap<usize, f32>,
    /// The number of rows the last "scrolled to the end" was sent for.
    pub end_sent: usize,
    /// The cell of the cursor as last drawn: when it moves out of view, the
    /// rows scroll to it.
    pub cursor: Option<(usize, usize)>,
    /// The cell being typed in as last drawn: its field takes the keyboard
    /// when it appears.
    pub editing: Option<(usize, usize)>,
    /// What the field of that cell shows while it is typed in: the plugin's
    /// answer comes after the next key.
    pub typed: Option<String>,
    /// The value the plugin last drew in it.
    pub drawn: String,
    /// The same for the filters of the columns, by column.
    pub filters: HashMap<usize, String>,
    pub drawn_filters: Vec<String>,
}

/// The scrollable of the rows of a grid.
pub fn scroll_id(key: &PanelKey, id: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("plugin-grid:{}:{}:{id}", key.0, key.1))
}

/// The grids of a tree of nodes.
pub fn all(node: &Node) -> Vec<&Grid> {
    match node {
        Node::Grid(grid) => vec![grid.as_ref()],
        Node::Column { children, .. } | Node::Row { children, .. } => {
            children.iter().flat_map(all).collect()
        }
        Node::Scroll { child, .. } => all(child),
        _ => Vec::new(),
    }
}

/// Where the rows must scroll to show a row (none: it is in view).
pub fn follow(row: usize, offset: f32, height: f32) -> Option<f32> {
    let height = if height > 0.0 { height } else { 400.0 };
    let top = row as f32 * ROW_HEIGHT;
    if top < offset {
        Some(top)
    } else if top + ROW_HEIGHT > offset + height {
        Some(top + ROW_HEIGHT - height)
    } else {
        None
    }
}

/// What happens to a grid, for the editor.
#[derive(Debug, Clone)]
pub enum GridMsg {
    Scrolled(f32, f32),
    ScrolledAcross(f32, f32),
    Press(usize, usize),
    Open(usize, usize),
    Sort(usize),
    ResizeStart(usize),
    EditInput(String),
    EditCommit,
    Filter(usize, String),
    FilterSubmit(usize),
    /// An item of the context menu of a cell.
    Menu(String, usize, usize),
}

pub type GridKey = (PanelKey, String);

/// The grid of that id in a tree of nodes.
pub fn find<'a>(node: &'a Node, id: &str) -> Option<&'a Grid> {
    match node {
        Node::Grid(grid) if grid.id == id => Some(grid),
        Node::Column { children, .. } | Node::Row { children, .. } => {
            children.iter().find_map(|c| find(c, id))
        }
        Node::Scroll { child, .. } => find(child, id),
        _ => None,
    }
}

fn width_of(grid: &Grid, view: Option<&GridView>, col: usize) -> f32 {
    view.and_then(|v| v.widths.get(&col).copied())
        .or(grid.columns.get(col).and_then(|c| c.width))
        .unwrap_or(DEFAULT_WIDTH)
        .max(MIN_WIDTH)
}

fn msg(key: &PanelKey, id: &str, m: GridMsg) -> Message {
    Message::Plugin(PluginMsg::Grid(
        key.0.clone(),
        key.1.clone(),
        id.to_string(),
        m,
    ))
}

/// Whether a cell is in the block picked.
fn in_block(selection: Option<GridSelection>, r: usize, c: usize) -> bool {
    selection.is_some_and(|s| {
        let (r0, r1) = (s.anchor.0.min(s.cursor.0), s.anchor.0.max(s.cursor.0));
        let (c0, c1) = (s.anchor.1.min(s.cursor.1), s.anchor.1.max(s.cursor.1));
        (r0..=r1).contains(&r) && (c0..=c1).contains(&c)
    })
}

pub fn view<'a>(
    key: &PanelKey,
    grid: &'a Grid,
    view: Option<&GridView>,
    dark: bool,
) -> Element<'a, Message> {
    let p = se_theme::palette(dark);
    let id = grid.id.as_str();
    let widths: Vec<f32> = (0..grid.columns.len())
        .map(|c| width_of(grid, view, c))
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + GUTTER;
    let gutter_cell = |label: String, color: iced::Color| {
        container(text(label).size(11.5).color(color))
            .width(Length::Fixed(GUTTER))
            .height(Length::Fixed(ROW_HEIGHT))
            .padding([0, 10])
            .center_y(Length::Fixed(ROW_HEIGHT))
            .align_x(iced::alignment::Horizontal::Right)
    };

    // The titles: a click sorts, the edge is dragged.
    let mut header = row![gutter_cell(String::new(), p.gutter)];
    for (c, column) in grid.columns.iter().enumerate() {
        let mut label = row![text(column.name.as_str())
            .size(12.5)
            .font(se_theme::UI_SEMIBOLD)
            .wrapping(iced::widget::text::Wrapping::None),]
        .spacing(6)
        .align_y(Alignment::Center);
        if !column.detail.is_empty() {
            label = label.push(
                text(column.detail.as_str())
                    .size(11)
                    .color(p.muted)
                    .wrapping(iced::widget::text::Wrapping::None),
            );
        }
        if let Some(asc) = column.sort {
            label = label.push(text(if asc { "▲" } else { "▼" }).size(10).color(p.accent));
        }
        header = header.push(
            button(label)
                .padding([0, 10])
                .width(Length::Fixed(widths[c] - HANDLE))
                .height(Length::Fixed(ROW_HEIGHT))
                .on_press(msg(key, id, GridMsg::Sort(c)))
                .style(
                    (se_theme::GhostButton {
                        dark,
                        active: column.sort.is_some(),
                    })
                    .style(),
                ),
        );
        header = header.push(
            iced::widget::mouse_area(
                container(Space::new())
                    .width(Length::Fixed(HANDLE))
                    .height(Length::Fixed(ROW_HEIGHT))
                    .style(move |_: &iced::Theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color {
                            a: 0.35,
                            ..p.muted
                        })),
                        ..container::Style::default()
                    }),
            )
            .on_press(msg(key, id, GridMsg::ResizeStart(c)))
            .interaction(iced::mouse::Interaction::ResizingHorizontally),
        );
    }
    let header = container(header).style(move |_: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(p.elevated)),
        ..container::Style::default()
    });

    let filters: Option<Element<'a, Message>> = grid.filters.then(|| {
        let mut line = row![gutter_cell(String::new(), p.gutter)];
        for (c, column) in grid.columns.iter().enumerate() {
            let k = key.clone();
            line = line.push(
                container(
                    text_input(
                        "",
                        view.and_then(|v| v.filters.get(&c))
                            .unwrap_or(&column.filter),
                    )
                    .on_input(move |t| msg(&k, id, GridMsg::Filter(c, t)))
                    .on_submit(msg(key, id, GridMsg::FilterSubmit(c)))
                    .size(12)
                    .padding([2, 6])
                    .style(se_theme::Field::new(dark).style()),
                )
                .width(Length::Fixed(widths[c]))
                .padding([2, 4]),
            );
        }
        line.into()
    });

    // Only the rows on screen (and a few around) are widgets.
    let count = grid.rows.len();
    let (offset, height) = view.map_or((0.0, 800.0), |v| {
        (
            v.scroll.0,
            if v.scroll.1 > 0.0 { v.scroll.1 } else { 800.0 },
        )
    });
    let first = ((offset / ROW_HEIGHT) as usize).saturating_sub(6);
    let last = (((offset + height) / ROW_HEIGHT) as usize + 6).min(count);
    let (across, across_w) = view.map_or((0.0, 2000.0), |v| {
        (
            v.across.0,
            if v.across.1 > 0.0 { v.across.1 } else { 2000.0 },
        )
    });
    let mut rows = column![Space::new().height(first as f32 * ROW_HEIGHT)];
    let focus = grid.selection.map(|s| s.cursor);
    for (r, grid_row) in grid.rows.iter().enumerate().take(last).skip(first) {
        let state = grid_row.state;
        let mut line = row![gutter_cell(
            if state == RowState::New {
                "+".to_string()
            } else {
                (r + 1).to_string()
            },
            if focus.is_some_and(|f| f.0 == r) || state == RowState::New {
                p.accent
            } else {
                p.gutter
            }
        )];
        let mut gap = 0.0f32;
        let mut x = 0.0f32;
        for (c, width) in widths.iter().enumerate() {
            let from = x;
            x += width;
            let typing = grid.editing.as_ref().filter(|e| e.row == r && e.col == c);
            let in_view = from + width >= across - 400.0 && from <= across + across_w + 400.0;
            if !in_view && typing.is_none() && focus != Some((r, c)) {
                gap += width;
                continue;
            }
            if gap > 0.0 {
                line = line.push(Space::new().width(Length::Fixed(gap)));
                gap = 0.0;
            }
            let cell = grid_row.cells.get(c);
            let content: Element<Message> = match (typing, cell) {
                (Some(editing), _) => text_input(
                    "",
                    view.and_then(|v| v.typed.as_deref())
                        .unwrap_or(&editing.value),
                )
                .id(edit_id())
                .on_input({
                    let k = key.clone();
                    move |t| msg(&k, id, GridMsg::EditInput(t))
                })
                .on_submit(msg(key, id, GridMsg::EditCommit))
                .size(12.5)
                .padding([2, 6])
                .style(se_theme::Field::new(dark).style())
                .into(),
                (None, Some(cell)) if cell.null => text("NULL")
                    .size(11.5)
                    .color(p.muted)
                    .font(se_theme::UI_ITALIC)
                    .into(),
                (None, Some(cell)) => text(one_line(&cell.text))
                    .size(12.5)
                    .color(if state == RowState::Deleted {
                        p.muted
                    } else {
                        p.text
                    })
                    .wrapping(iced::widget::text::Wrapping::None)
                    .into(),
                (None, None) => Space::new().into(),
            };
            let edited = cell.is_some_and(|c| c.edited);
            let focused = focus == Some((r, c));
            let picked = in_block(grid.selection, r, c);
            let base = container(content)
                .width(Length::Fixed(*width))
                .height(Length::Fixed(ROW_HEIGHT))
                .padding(if typing.is_some() { [0, 3] } else { [0, 10] })
                .center_y(Length::Fixed(ROW_HEIGHT))
                .clip(true)
                .style(move |_: &iced::Theme| container::Style {
                    background: if edited {
                        Some(iced::Background::Color(iced::Color {
                            a: 0.28,
                            ..p.accent
                        }))
                    } else {
                        (focused || picked).then_some(iced::Background::Color(iced::Color {
                            a: if picked && !focused { 0.18 } else { 0.10 },
                            ..p.accent
                        }))
                    },
                    border: if focused {
                        iced::Border {
                            color: p.accent,
                            width: 1.5,
                            radius: 0.0.into(),
                        }
                    } else {
                        iced::Border::default()
                    },
                    ..container::Style::default()
                });
            let hit =
                iced::widget::mouse_area(base).on_double_click(msg(key, id, GridMsg::Open(r, c)));
            let element: Element<Message> = if grid.menu.is_empty() {
                hit.into()
            } else {
                let items = grid.menu.clone();
                let k = key.clone();
                crate::pointer_area::context_menu(hit, move || {
                    use crate::menu_bar::{disabled_item, item};
                    let mut list = column![].spacing(1).padding(6).width(Length::Fixed(240.0));
                    for entry in &items {
                        list = list.push(if entry.enabled {
                            item(
                                entry.label.clone(),
                                entry.key.clone(),
                                dark,
                                msg(&k, id, GridMsg::Menu(entry.action.clone(), r, c)),
                            )
                        } else {
                            disabled_item(entry.label.clone(), dark)
                        });
                    }
                    container(list).style(se_theme::card(dark)).into()
                })
            };
            // A click, left or right, picks the cell (the menu and the
            // double click still get theirs).
            let press = msg(key, id, GridMsg::Press(r, c));
            let element = crate::pointer_area::pointer_area(element)
                .watching()
                .on_press({
                    let press = press.clone();
                    move |_| press.clone()
                })
                .on_right_press(press);
            line = line.push(element);
        }
        if gap > 0.0 {
            line = line.push(Space::new().width(Length::Fixed(gap)));
        }
        rows = rows.push(
            container(line).style(move |_: &iced::Theme| container::Style {
                background: match state {
                    RowState::Deleted => Some(iced::Background::Color(iced::Color {
                        a: 0.16,
                        ..se_theme::error_color(dark)
                    })),
                    RowState::New => Some(iced::Background::Color(iced::Color {
                        a: 0.12,
                        ..p.accent
                    })),
                    _ => (r % 2 == 1).then_some(iced::Background::Color(iced::Color {
                        a: 0.35,
                        ..p.elevated
                    })),
                },
                ..container::Style::default()
            }),
        );
    }
    rows = rows.push(Space::new().height(count.saturating_sub(last) as f32 * ROW_HEIGHT));

    let k = key.clone();
    let vertical = scrollable(rows)
        .id(scroll_id(key, id))
        .on_scroll(move |v| {
            msg(
                &k,
                id,
                GridMsg::Scrolled(v.absolute_offset().y, v.bounds().height),
            )
        })
        .style(se_theme::Scrollbar { dark }.style())
        .width(Length::Fixed(total))
        .height(Length::Fill);
    let mut top = column![header].width(Length::Fixed(total));
    if let Some(filters) = filters {
        top = top.push(filters);
    }
    let k = key.clone();
    let across_scroll = scrollable(top.push(vertical))
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new(),
        ))
        .on_scroll(move |v| {
            msg(
                &k,
                id,
                GridMsg::ScrolledAcross(v.absolute_offset().x, v.bounds().width),
            )
        })
        .style(se_theme::Scrollbar { dark }.style())
        .width(Length::Fill)
        .height(grid.height.map_or(Length::Fill, Length::Fixed));
    across_scroll.into()
}

/// A value on one line: what follows a line break is left out.
fn one_line(value: &str) -> String {
    match value.split_once('\n') {
        Some((first, _)) => format!("{first} …"),
        None => value.to_string(),
    }
}

/// The field of the cell being typed in (it takes the keyboard).
pub fn edit_id() -> iced::widget::Id {
    iced::widget::Id::new("plugin-grid-edit")
}

/// What a key does to a grid that has the keyboard (none: not for it).
pub fn key(grid: &Grid, key: &str, ctrl: bool, shift: bool) -> Option<GridEvent> {
    if grid.editing.is_some() {
        return (key == "Escape").then_some(GridEvent::Cancel);
    }
    let rows = grid.rows.len();
    let cols = grid.columns.len();
    if rows == 0 || cols == 0 {
        return None;
    }
    let selection = grid.selection.unwrap_or(GridSelection {
        anchor: (0, 0),
        cursor: (0, 0),
    });
    let (r, c) = selection.cursor;
    let moved = |to: (usize, usize)| {
        let to = (to.0.min(rows - 1), to.1.min(cols - 1));
        GridEvent::Select {
            anchor: if shift { selection.anchor } else { to },
            cursor: to,
        }
    };
    match key {
        "ArrowUp" => Some(moved((r.saturating_sub(1), c))),
        "ArrowDown" => Some(moved((r + 1, c))),
        "ArrowLeft" => Some(moved((r, c.saturating_sub(1)))),
        "ArrowRight" => Some(moved((r, c + 1))),
        "PageUp" => Some(moved((r.saturating_sub(20), c))),
        "PageDown" => Some(moved((r + 20, c))),
        "Home" if ctrl => Some(moved((0, 0))),
        "End" if ctrl => Some(moved((rows - 1, cols - 1))),
        "Home" => Some(moved((r, 0))),
        "End" => Some(moved((r, cols - 1))),
        // Tab goes on to the next cell (the next row after the last
        // column), Shift+Tab back.
        "Tab" => {
            let to = if shift {
                match (r, c) {
                    (_, c) if c > 0 => (r, c - 1),
                    (r, _) if r > 0 => (r - 1, cols - 1),
                    _ => (r, c),
                }
            } else if c + 1 < cols {
                (r, c + 1)
            } else if r + 1 < rows {
                (r + 1, 0)
            } else {
                (r, c)
            };
            Some(GridEvent::Select {
                anchor: to,
                cursor: to,
            })
        }
        "Enter" | "F2" => Some(GridEvent::Open { row: r, col: c }),
        "Delete" => Some(GridEvent::Delete),
        "a" | "A" if ctrl => Some(GridEvent::Select {
            anchor: (0, 0),
            cursor: (rows - 1, cols - 1),
        }),
        "c" | "C" if ctrl => Some(GridEvent::Copy),
        _ => None,
    }
}

/// The item of the menu of a grid that a key does, on the cell of the
/// cursor: its action, row and column.
pub fn menu_key(
    grid: &Grid,
    key: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
) -> Option<(String, usize, usize)> {
    if grid.editing.is_some() || grid.rows.is_empty() {
        return None;
    }
    let (row, col) = grid.selection.map_or((0, 0), |s| s.cursor);
    grid.menu
        .iter()
        .filter(|item| item.enabled)
        .find(|item| {
            item.key
                .as_deref()
                .is_some_and(|spec| super::key_matches(spec, key, ctrl, alt, shift))
        })
        .map(|item| (item.action.clone(), row, col))
}

/// How a column is wider once dragged by `dx`.
pub fn dragged(width: f32, dx: f32) -> f32 {
    (width + dx).max(MIN_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ardoise_plugin::ui::{grid, GridCell, GridColumn, GridEditing, GridRow};

    fn three_by_two() -> Grid {
        grid(
            "g",
            (0..2).map(|i| GridColumn {
                name: format!("c{i}"),
                ..GridColumn::default()
            }),
            (0..3).map(|_| GridRow {
                cells: vec![GridCell::default(), GridCell::default()],
                ..GridRow::default()
            }),
        )
    }

    #[test]
    fn the_arrows_move_in_the_grid_and_shift_makes_a_block() {
        let mut g = three_by_two();
        assert_eq!(
            key(&g, "ArrowDown", false, false),
            Some(GridEvent::Select {
                anchor: (1, 0),
                cursor: (1, 0)
            })
        );
        g.selection = Some(GridSelection {
            anchor: (1, 0),
            cursor: (1, 0),
        });
        assert_eq!(
            key(&g, "ArrowRight", false, true),
            Some(GridEvent::Select {
                anchor: (1, 0),
                cursor: (1, 1)
            })
        );
        // Kept inside the grid.
        assert_eq!(
            key(&g, "PageDown", false, false),
            Some(GridEvent::Select {
                anchor: (2, 0),
                cursor: (2, 0)
            })
        );
        assert_eq!(
            key(&g, "Enter", false, false),
            Some(GridEvent::Open { row: 1, col: 0 })
        );
        // Tab: the next cell, then the next row; Shift+Tab back.
        let at = |g: &mut Grid, cell: (usize, usize)| {
            g.selection = Some(GridSelection {
                anchor: cell,
                cursor: cell,
            })
        };
        let to = |cell: (usize, usize)| {
            Some(GridEvent::Select {
                anchor: cell,
                cursor: cell,
            })
        };
        assert_eq!(key(&g, "Tab", false, false), to((1, 1)));
        at(&mut g, (1, 1));
        assert_eq!(key(&g, "Tab", false, false), to((2, 0)));
        at(&mut g, (2, 0));
        assert_eq!(key(&g, "Tab", false, true), to((1, 1)));
        at(&mut g, (1, 0));
        assert_eq!(key(&g, "c", true, false), Some(GridEvent::Copy));
        assert_eq!(key(&g, "x", false, false), None);
        // Typing in a cell: only Escape is the grid's.
        g.editing = Some(GridEditing {
            row: 1,
            col: 0,
            value: "a".into(),
        });
        assert_eq!(key(&g, "ArrowDown", false, false), None);
        assert_eq!(key(&g, "Escape", false, false), Some(GridEvent::Cancel));
    }

    #[test]
    fn a_grid_is_found_in_its_panel_and_a_block_holds_its_cells() {
        let node = Node::Column {
            children: vec![Node::Scroll {
                child: Box::new(three_by_two().into()),
                bottom: false,
                height: None,
            }],
            spacing: None,
            padding: None,
            width: None,
        };
        assert!(find(&node, "g").is_some());
        assert!(find(&node, "h").is_none());
        let s = Some(GridSelection {
            anchor: (2, 1),
            cursor: (0, 0),
        });
        assert!(in_block(s, 1, 1) && in_block(s, 0, 0));
        assert!(!in_block(s, 3, 0));
        assert_eq!(dragged(100.0, -90.0), MIN_WIDTH);
        // The cursor moved under the rows shown: they scroll just enough.
        assert_eq!(follow(5, 0.0, 10.0 * ROW_HEIGHT), None);
        assert_eq!(follow(12, 0.0, 10.0 * ROW_HEIGHT), Some(3.0 * ROW_HEIGHT));
        assert_eq!(
            follow(2, 5.0 * ROW_HEIGHT, 10.0 * ROW_HEIGHT),
            Some(2.0 * ROW_HEIGHT)
        );
        assert_eq!(all(&node).len(), 1);
        assert_eq!(one_line("a\nb"), "a …");
    }
}
