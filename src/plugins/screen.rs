//! A terminal screen of a plugin: the grid of cells it sent, drawn on a
//! canvas. While it has the focus, the keys become the bytes a terminal
//! sends; they, the size the screen has, the wheel and a paste go back to
//! the plugin. A selection is copied, a `file:line` opened with a click.

use iced::keyboard::{self, key::Named, Key};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Frame, Geometry, Text};
use iced::{Color, Element, Length, Pixels, Point, Rectangle, Size, Theme};
use serde_json::json;
use simpleedit_plugin::ui::{MouseMode, Run, Screen as Grid, ScreenInput, TermColor};

use super::{PanelKey, PluginMsg};
use crate::app::Message;
use crate::theme;

pub const FONT_SIZE: f32 = 13.0;
/// JetBrains Mono's advance is 0.6 em.
const CELL_W: f32 = FONT_SIZE * 0.6;
const CELL_H: f32 = 18.0;
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 6.0;

/// The bytes that tell a program the mouse did something: `button` is the
/// code of the xterm protocol (0 left, 64 and 65 the wheel, +32 for motion),
/// `release` ends a press; the cell is counted from 0.
fn mouse_report(sgr: bool, button: u8, release: bool, (row, col): (u16, u16)) -> Vec<u8> {
    let (x, y) = (col + 1, row + 1);
    if sgr {
        format!("\x1b[<{button};{x};{y}{}", if release { 'm' } else { 'M' }).into_bytes()
    } else {
        // The older way: a release has no button, and the places stop where
        // a byte does.
        let code = if release { 3 } else { button };
        let byte = |n: u16| (n.min(223) + 32) as u8;
        vec![0x1b, b'[', b'M', code + 32, byte(x), byte(y)]
    }
}

/// The word of a row of the screen at a column, without the punctuation
/// that ends a sentence.
fn word_at(row: &str, col: usize) -> Option<String> {
    let chars: Vec<char> = row.chars().collect();
    let at = col.min(chars.len().checked_sub(1)?);
    if chars[at].is_whitespace() {
        return None;
    }
    let mut start = at;
    while start > 0 && !chars[start - 1].is_whitespace() {
        start -= 1;
    }
    let mut end = at;
    while end + 1 < chars.len() && !chars[end + 1].is_whitespace() {
        end += 1;
    }
    let word: String = chars[start..=end].iter().collect();
    Some(
        word.trim_matches(|c| matches!(c, ',' | ';' | '\'' | '"' | '`' | '(' | ')' | '[' | ']'))
            .to_string(),
    )
    .filter(|w| !w.is_empty())
}

// ─── Keys ───────────────────────────────────────────────────────────────────

/// The bytes a terminal sends for a key press, if it sends any.
pub fn encode_key(
    key: &Key,
    modifiers: keyboard::Modifiers,
    text: Option<&str>,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    let (shift, alt, ctrl) = (modifiers.shift(), modifiers.alt(), modifiers.control());
    // xterm's modifier parameter: 1 + shift + 2 alt + 4 ctrl.
    let param = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    let csi = |final_byte: char| -> Vec<u8> {
        if param > 1 {
            format!("\x1b[1;{param}{final_byte}").into_bytes()
        } else if application_cursor {
            format!("\x1bO{final_byte}").into_bytes()
        } else {
            format!("\x1b[{final_byte}").into_bytes()
        }
    };
    let tilde = |n: u8| -> Vec<u8> {
        if param > 1 {
            format!("\x1b[{n};{param}~").into_bytes()
        } else {
            format!("\x1b[{n}~").into_bytes()
        }
    };
    let function = |n: u8| -> Vec<u8> {
        match n {
            1..=4 => {
                let letter = (b'P' + n - 1) as char;
                if param > 1 {
                    format!("\x1b[1;{param}{letter}").into_bytes()
                } else {
                    format!("\x1bO{letter}").into_bytes()
                }
            }
            _ => tilde(match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                _ => 24,
            }),
        }
    };

    if let Key::Named(named) = key {
        let bytes = match named {
            Named::Enter => vec![b'\r'],
            Named::Tab if shift => b"\x1b[Z".to_vec(),
            Named::Tab => vec![b'\t'],
            Named::Backspace if alt => vec![0x1b, 0x7f],
            Named::Backspace => vec![0x7f],
            Named::Escape => vec![0x1b],
            Named::Space if ctrl => vec![0],
            Named::Space => b" ".to_vec(),
            Named::ArrowUp => csi('A'),
            Named::ArrowDown => csi('B'),
            Named::ArrowRight => csi('C'),
            Named::ArrowLeft => csi('D'),
            Named::Home => csi('H'),
            Named::End => csi('F'),
            Named::Insert => tilde(2),
            Named::Delete => tilde(3),
            Named::PageUp => tilde(5),
            Named::PageDown => tilde(6),
            Named::F1 => function(1),
            Named::F2 => function(2),
            Named::F3 => function(3),
            Named::F4 => function(4),
            Named::F5 => function(5),
            Named::F6 => function(6),
            Named::F7 => function(7),
            Named::F8 => function(8),
            Named::F9 => function(9),
            Named::F10 => function(10),
            Named::F11 => function(11),
            Named::F12 => function(12),
            _ => return None,
        };
        return Some(bytes);
    }

    let Key::Character(typed) = key else {
        return None;
    };
    let mut chars = typed.chars();
    let first = chars.next()?;
    // Control + a letter (or one of a few symbols) is a control character.
    if ctrl && chars.next().is_none() {
        let control = match first.to_ascii_lowercase() {
            c @ 'a'..='z' => Some(c as u8 - b'a' + 1),
            '@' | ' ' | '2' => Some(0),
            '[' | '3' => Some(0x1b),
            '\\' | '4' => Some(0x1c),
            ']' | '5' => Some(0x1d),
            '^' | '6' => Some(0x1e),
            '_' | '-' | '7' => Some(0x1f),
            _ => None,
        };
        if let Some(byte) = control {
            let mut out = Vec::with_capacity(2);
            if alt {
                out.push(0x1b);
            }
            out.push(byte);
            return Some(out);
        }
    }
    // What the layout produced (dead keys, AltGr…), else the key itself.
    let produced = text
        .filter(|t| !t.is_empty() && !t.chars().any(char::is_control))
        .unwrap_or(typed.as_str());
    let mut out = Vec::new();
    if alt && !ctrl {
        out.push(0x1b);
    }
    out.extend_from_slice(produced.as_bytes());
    Some(out)
}

// ─── Colors ─────────────────────────────────────────────────────────────────

/// The 16 ANSI colors.
fn ansi(index: u8, dark: bool) -> Color {
    const DARK: [(u8, u8, u8); 16] = [
        (0x1b, 0x1d, 0x23),
        (0xf0, 0x6b, 0x6b),
        (0x8f, 0xd1, 0x8f),
        (0xe6, 0xc0, 0x6a),
        (0x6e, 0xa8, 0xff),
        (0xc6, 0x8a, 0xe8),
        (0x5e, 0xcf, 0xd1),
        (0xd0, 0xd3, 0xda),
        (0x5c, 0x61, 0x70),
        (0xff, 0x8a, 0x8a),
        (0xa8, 0xe8, 0xa8),
        (0xf5, 0xd5, 0x8a),
        (0x8f, 0xc0, 0xff),
        (0xdb, 0xa6, 0xf5),
        (0x84, 0xe3, 0xe5),
        (0xf2, 0xf3, 0xf6),
    ];
    const LIGHT: [(u8, u8, u8); 16] = [
        (0x1b, 0x1d, 0x23),
        (0xc6, 0x28, 0x28),
        (0x2e, 0x7d, 0x32),
        (0x9a, 0x6a, 0x00),
        (0x1b, 0x5f, 0xc6),
        (0x8e, 0x3a, 0xb8),
        (0x00, 0x7c, 0x85),
        (0x5c, 0x61, 0x70),
        (0x80, 0x85, 0x92),
        (0xd9, 0x3a, 0x3a),
        (0x3d, 0x9a, 0x42),
        (0xb0, 0x7d, 0x00),
        (0x2e, 0x74, 0xe0),
        (0xa4, 0x4f, 0xd0),
        (0x00, 0x96, 0xa0),
        (0x1b, 0x1d, 0x23),
    ];
    let (r, g, b) = if dark { DARK } else { LIGHT }[usize::from(index & 15)];
    Color::from_rgb8(r, g, b)
}

/// One of the 256 indexed colors: the 16 ANSI ones, a 6×6×6 cube, a gray ramp.
fn indexed(index: u8, dark: bool) -> Color {
    match index {
        0..=15 => ansi(index, dark),
        16..=231 => {
            let n = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            Color::from_rgb8(level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        _ => {
            let v = 8 + 10 * (index - 232);
            Color::from_rgb8(v, v, v)
        }
    }
}

// ─── The canvas ─────────────────────────────────────────────────────────────

/// The terminal on a canvas.
/// A selection by dragging, in cells (row, column) of the visible screen.
#[derive(Default)]
struct Drag {
    anchor: Option<(u16, u16)>,
    head: Option<(u16, u16)>,
    dragging: bool,
    /// A program that asked for the mouse has the button down.
    reported: bool,
    modifiers: keyboard::Modifiers,
}

impl Drag {
    /// The selected cells in reading order, ends included.
    fn range(&self) -> Option<((u16, u16), (u16, u16))> {
        let (a, b) = (self.anchor?, self.head?);
        if a == b {
            return None;
        }
        Some(if a <= b { (a, b) } else { (b, a) })
    }
}

fn grid_size(bounds: Size) -> (u16, u16) {
    let cols = ((bounds.width - 2.0 * PAD_X) / CELL_W).floor().max(2.0) as u16;
    let rows = ((bounds.height - 2.0 * PAD_Y) / CELL_H).floor().max(1.0) as u16;
    (rows, cols)
}

fn cell_at(point: Point, rows: u16, cols: u16) -> (u16, u16) {
    let col = ((point.x - PAD_X) / CELL_W).floor().max(0.0) as u16;
    let row = ((point.y - PAD_Y) / CELL_H).floor().max(0.0) as u16;
    (row.min(rows - 1), col.min(cols - 1))
}

/// The screen of a panel of a plugin.
pub fn view<'a>(key: &PanelKey, grid: &'a Grid, dark: bool, focused: bool) -> Element<'a, Message> {
    Canvas::new(ScreenView {
        key: key.clone(),
        grid,
        dark,
        focused,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

struct ScreenView<'a> {
    key: PanelKey,
    grid: &'a Grid,
    dark: bool,
    focused: bool,
}

impl ScreenView<'_> {
    fn send(&self, input: ScreenInput) -> Message {
        Message::Plugin(PluginMsg::Ui(
            self.key.0.clone(),
            self.key.1.clone(),
            self.grid.id.clone(),
            Some(json!(input)),
        ))
    }

    fn write(&self, bytes: Vec<u8>) -> Message {
        self.send(ScreenInput::Input { bytes })
    }

    fn focus(&self, on: bool) -> Message {
        Message::Plugin(PluginMsg::ScreenFocus(
            self.key.0.clone(),
            self.key.1.clone(),
            self.grid.id.clone(),
            on,
        ))
    }

    /// The text of a line from a column to another (end excluded).
    fn text_between(&self, row: u16, from: u16, to: u16) -> String {
        let Some(runs) = self.grid.lines.get(row as usize) else {
            return String::new();
        };
        let mut cells: Vec<char> = vec![' '; self.grid.size.1 as usize];
        for run in runs {
            let mut col = run.col as usize;
            for c in run.text.chars() {
                if let Some(cell) = cells.get_mut(col) {
                    *cell = c;
                }
                col += if run.wide { 2 } else { 1 };
            }
        }
        let (from, to) = (from as usize, (to as usize).min(cells.len()));
        if from >= to {
            return String::new();
        }
        cells[from..to]
            .iter()
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    /// The text selected, lines joined.
    fn selected_text(&self, ((r0, c0), (r1, c1)): ((u16, u16), (u16, u16))) -> String {
        let cols = self.grid.size.1;
        (r0..=r1)
            .map(|row| {
                let from = if row == r0 { c0 } else { 0 };
                let to = if row == r1 {
                    c1.saturating_add(1)
                } else {
                    cols
                };
                self.text_between(row, from, to)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A run of a selection by dragging also remembers the size last sent.
#[derive(Default)]
struct State {
    drag: Drag,
    sent: Option<(u16, u16)>,
}

impl canvas::Program<Message> for ScreenView<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let (rows, cols) = grid_size(bounds.size());
        // The program gets the room the screen has.
        if (rows, cols) != self.grid.size && state.sent != Some((rows, cols)) {
            state.sent = Some((rows, cols));
            return Some(canvas::Action::publish(
                self.send(ScreenInput::Resize { rows, cols }),
            ));
        }
        let drag = &mut state.drag;
        // A program that asked for the mouse (vim, htop, tmux) gets it; the
        // Shift key keeps the selection for the terminal.
        let mode = self.grid.mouse;
        let sgr = self.grid.mouse_sgr;
        let reporting = mode != MouseMode::None && !drag.modifiers.shift();
        match event {
            iced::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                drag.modifiers = *m;
                None
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if reporting && self.focused && cursor.position_in(bounds).is_some() =>
            {
                let cell = cell_at(cursor.position_in(bounds)?, rows, cols);
                drag.reported = true;
                Some(
                    canvas::Action::publish(self.write(mouse_report(sgr, 0, false, cell)))
                        .and_capture(),
                )
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if drag.reported =>
            {
                drag.reported = false;
                if mode != MouseMode::Press {
                    if let Some(at) = cursor.position_in(bounds) {
                        let cell = cell_at(at, rows, cols);
                        return Some(
                            canvas::Action::publish(self.write(mouse_report(sgr, 0, true, cell)))
                                .and_capture(),
                        );
                    }
                }
                Some(canvas::Action::capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position })
                if drag.reported
                    && matches!(mode, MouseMode::ButtonMotion | MouseMode::AnyMotion) =>
            {
                let at = Point::new(position.x - bounds.x, position.y - bounds.y);
                let report = mouse_report(sgr, 32, false, cell_at(at, rows, cols));
                Some(canvas::Action::publish(self.write(report)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if drag.modifiers.command() && cursor.position_in(bounds).is_some() =>
            {
                // Command (Ctrl elsewhere) and a click on `file:line`: opens it.
                let (row, col) = cell_at(cursor.position_in(bounds)?, rows, cols);
                let line = self.text_between(row, 0, cols);
                let place = word_at(&line, col as usize).unwrap_or(line);
                let dir = self.grid.dir.clone().unwrap_or_default();
                Some(
                    canvas::Action::publish(Message::Plugin(PluginMsg::ScreenOpen(place, dir)))
                        .and_capture(),
                )
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(at) = cursor.position_in(bounds) {
                    let cell = cell_at(at, rows, cols);
                    *drag = Drag {
                        anchor: Some(cell),
                        head: Some(cell),
                        dragging: true,
                        modifiers: drag.modifiers,
                        reported: false,
                    };
                    let action = if self.focused {
                        canvas::Action::request_redraw()
                    } else {
                        canvas::Action::publish(self.focus(true))
                    };
                    return Some(action.and_capture());
                }
                // A click elsewhere takes the keyboard back (the click goes on).
                *drag = Drag::default();
                self.focused
                    .then(|| canvas::Action::publish(self.focus(false)))
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if drag.dragging => {
                let at = Point::new(position.x - bounds.x, position.y - bounds.y);
                drag.head = Some(cell_at(at, rows, cols));
                Some(canvas::Action::request_redraw())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if drag.dragging =>
            {
                drag.dragging = false;
                if drag.range().is_none() {
                    *drag = Drag::default();
                }
                Some(canvas::Action::request_redraw())
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                cursor.position_in(bounds)?;
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * 3.0,
                    mouse::ScrollDelta::Pixels { y, .. } => y / CELL_H,
                };
                let lines = lines.round() as i32;
                if lines == 0 {
                    return None;
                }
                let message = if reporting {
                    let cell = cell_at(cursor.position_in(bounds)?, rows, cols);
                    let button = if lines > 0 { 64 } else { 65 };
                    let one = mouse_report(sgr, button, false, cell);
                    self.write(one.repeat(lines.unsigned_abs().min(10) as usize))
                } else if self.grid.alternate {
                    // A full-screen program (less, vim) has no history: the
                    // wheel moves its cursor instead.
                    let key: &[u8] = if lines > 0 { b"\x1b[A" } else { b"\x1b[B" };
                    self.write(key.repeat(lines.unsigned_abs().min(10) as usize))
                } else {
                    self.send(ScreenInput::Scroll { lines })
                };
                *drag = Drag::default();
                Some(canvas::Action::publish(message).and_capture())
            }
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                text,
                ..
            }) if self.focused => {
                // Copy and paste: Cmd+C / Cmd+V on macOS, Ctrl+Shift+C / V
                // elsewhere (plain Ctrl+C belongs to the shell).
                let chord = if cfg!(target_os = "macos") {
                    modifiers.command() && !modifiers.control()
                } else {
                    modifiers.control() && modifiers.shift()
                };
                if chord {
                    if let Key::Character(c) = key {
                        match c.to_lowercase().as_str() {
                            "c" => {
                                let selected = drag.range().map(|r| self.selected_text(r));
                                return Some(match selected {
                                    Some(text) if !text.is_empty() => canvas::Action::publish(
                                        Message::Plugin(PluginMsg::ScreenCopy(text)),
                                    )
                                    .and_capture(),
                                    _ => canvas::Action::capture(),
                                });
                            }
                            "v" => {
                                let paste = PluginMsg::ScreenPaste(
                                    self.key.0.clone(),
                                    self.key.1.clone(),
                                    self.grid.id.clone(),
                                    self.grid.bracketed_paste,
                                );
                                return Some(
                                    canvas::Action::publish(Message::Plugin(paste)).and_capture(),
                                );
                            }
                            _ => {}
                        }
                    }
                }
                if cfg!(target_os = "macos") && modifiers.command() {
                    // Other Cmd chords are the app's.
                    return None;
                }
                let bytes = encode_key(key, *modifiers, text.as_deref(), self.grid.app_cursor)?;
                *drag = Drag::default();
                Some(canvas::Action::publish(self.write(bytes)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let p = theme::palette(self.dark);
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), p.panel);
        let selection = state.drag.range();
        let selected = |row: u16, col: u16| {
            selection.is_some_and(|(start, end)| (row, col) >= start && (row, col) <= end)
        };
        let (rows, cols) = grid_size(bounds.size());
        let needle = self.grid.highlight.to_lowercase();
        for (row, runs) in self.grid.lines.iter().enumerate().take(rows as usize) {
            let row = row as u16;
            let y = PAD_Y + f32::from(row) * CELL_H;
            // The columns of this line where the search text is.
            let marks: Vec<(u16, u16)> = if needle.is_empty() {
                Vec::new()
            } else {
                let line = self.text_between(row, 0, cols).to_lowercase();
                line.match_indices(&needle)
                    .map(|(at, m)| {
                        let start = line[..at].chars().count() as u16;
                        (start, start + m.chars().count() as u16)
                    })
                    .collect()
            };
            for run in runs {
                draw_run(&mut frame, run, y, self.dark, |col| {
                    if selected(row, col) {
                        Some((p.accent_soft, p.text))
                    } else if marks.iter().any(|(a, b)| (*a..*b).contains(&col)) {
                        Some((p.accent, p.panel))
                    } else {
                        None
                    }
                });
            }
            // Selected blanks (between the runs) show too.
            if let Some(((r0, c0), (r1, c1))) = selection {
                if (r0..=r1).contains(&row) {
                    let from = if row == r0 { c0 } else { 0 };
                    let to = if row == r1 {
                        c1
                    } else {
                        cols.saturating_sub(1)
                    };
                    for col in from..=to {
                        let covered = runs.iter().any(|r| {
                            let width = if r.wide {
                                2
                            } else {
                                r.text.chars().count() as u16
                            };
                            (r.col..r.col + width).contains(&col)
                        });
                        if !covered {
                            frame.fill_rectangle(
                                Point::new(PAD_X + f32::from(col) * CELL_W, y),
                                Size::new(CELL_W, CELL_H),
                                p.accent_soft,
                            );
                        }
                    }
                }
            }
        }
        if let Some((row, col)) = self.grid.cursor {
            let at = Point::new(
                PAD_X + f32::from(col) * CELL_W,
                PAD_Y + f32::from(row) * CELL_H,
            );
            if self.focused {
                frame.fill_rectangle(at, Size::new(CELL_W, CELL_H), Color { a: 0.55, ..p.text });
            } else {
                frame.stroke_rectangle(
                    Point::new(at.x + 0.5, at.y + 0.5),
                    Size::new(CELL_W - 1.0, CELL_H - 1.0),
                    canvas::Stroke::default().with_color(p.muted),
                );
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }
}

fn term_color(c: Option<TermColor>, default: Color, dark: bool) -> Color {
    match c {
        None => default,
        Some(TermColor::Idx(n)) => indexed(n, dark),
        Some(TermColor::Rgb(r, g, b)) => Color::from_rgb8(r, g, b),
    }
}

/// Draws a run, cell by cell where a cell is selected or marked (`over`
/// gives its background and its ink), in one text otherwise.
fn draw_run(
    frame: &mut Frame,
    run: &Run,
    y: f32,
    dark: bool,
    over: impl Fn(u16) -> Option<(Color, Color)>,
) {
    let p = theme::palette(dark);
    let mut fg = term_color(run.fg, p.text, dark);
    let mut bg = run.bg.map(|b| term_color(Some(b), p.panel, dark));
    if run.inverse {
        let shown_bg = bg.unwrap_or(p.panel);
        bg = Some(fg);
        fg = shown_bg;
    }
    if run.dim {
        fg.a = 0.6;
    }
    let width = if run.wide { 2 } else { 1 };
    let mut font = theme::MONO;
    if run.bold {
        font.weight = iced::font::Weight::Medium;
    }
    let glyphs: Vec<char> = run.text.chars().collect();
    let plain = (0..glyphs.len()).all(|i| over(run.col + (i * width) as u16).is_none());
    let x0 = PAD_X + f32::from(run.col) * CELL_W;
    let text = |frame: &mut Frame, content: String, x: f32, color: Color| {
        frame.fill_text(Text {
            content,
            position: Point::new(x, y),
            color,
            size: Pixels(FONT_SIZE),
            line_height: iced::widget::text::LineHeight::Absolute(Pixels(CELL_H)),
            font,
            shaping: iced::widget::text::Shaping::Advanced,
            ..Text::default()
        });
    };
    if let Some(color) = bg {
        frame.fill_rectangle(
            Point::new(x0, y),
            Size::new((glyphs.len() * width) as f32 * CELL_W, CELL_H),
            color,
        );
    }
    if plain {
        text(frame, run.text.clone(), x0, fg);
    } else {
        for (i, c) in glyphs.iter().enumerate() {
            let col = run.col + (i * width) as u16;
            let x = PAD_X + f32::from(col) * CELL_W;
            let ink = match over(col) {
                Some((back, ink)) => {
                    frame.fill_rectangle(
                        Point::new(x, y),
                        Size::new(width as f32 * CELL_W, CELL_H),
                        back,
                    );
                    ink
                }
                None => fg,
            };
            text(frame, c.to_string(), x, ink);
        }
    }
    if run.underline {
        frame.fill_rectangle(
            Point::new(x0, y + CELL_H - 2.0),
            Size::new((glyphs.len() * width) as f32 * CELL_W, 1.0),
            fg,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::Modifiers;

    fn key(c: &str) -> Key {
        Key::Character(c.into())
    }

    fn bytes(key: Key, modifiers: Modifiers, text: Option<&str>) -> Option<Vec<u8>> {
        encode_key(&key, modifiers, text, false)
    }

    #[test]
    fn typed_characters_go_through_as_utf8() {
        assert_eq!(
            bytes(key("a"), Modifiers::empty(), Some("a")),
            Some(b"a".to_vec())
        );
        assert_eq!(
            bytes(key("é"), Modifiers::empty(), Some("é")),
            Some("é".as_bytes().to_vec())
        );
        // The layout's own output wins (Shift, AltGr…).
        assert_eq!(
            bytes(key("a"), Modifiers::SHIFT, Some("A")),
            Some(b"A".to_vec())
        );
    }

    #[test]
    fn control_letters_are_control_characters() {
        assert_eq!(bytes(key("c"), Modifiers::CTRL, None), Some(vec![3]));
        assert_eq!(bytes(key("d"), Modifiers::CTRL, None), Some(vec![4]));
        assert_eq!(bytes(key("z"), Modifiers::CTRL, None), Some(vec![26]));
        assert_eq!(bytes(key("["), Modifiers::CTRL, None), Some(vec![0x1b]));
    }

    #[test]
    fn alt_prefixes_an_escape() {
        assert_eq!(
            bytes(key("b"), Modifiers::ALT, Some("b")),
            Some(vec![0x1b, b'b'])
        );
        assert_eq!(
            bytes(Key::Named(Named::Backspace), Modifiers::ALT, None),
            Some(vec![0x1b, 0x7f])
        );
    }

    #[test]
    fn named_keys_send_their_sequences() {
        let named = |n| bytes(Key::Named(n), Modifiers::empty(), None);
        assert_eq!(named(Named::Enter), Some(b"\r".to_vec()));
        assert_eq!(named(Named::Backspace), Some(vec![0x7f]));
        assert_eq!(named(Named::Tab), Some(b"\t".to_vec()));
        assert_eq!(named(Named::Escape), Some(vec![0x1b]));
        assert_eq!(named(Named::ArrowUp), Some(b"\x1b[A".to_vec()));
        assert_eq!(named(Named::Delete), Some(b"\x1b[3~".to_vec()));
        assert_eq!(named(Named::PageUp), Some(b"\x1b[5~".to_vec()));
        assert_eq!(named(Named::F1), Some(b"\x1bOP".to_vec()));
        assert_eq!(named(Named::F5), Some(b"\x1b[15~".to_vec()));
        assert_eq!(
            bytes(Key::Named(Named::Tab), Modifiers::SHIFT, None),
            Some(b"\x1b[Z".to_vec())
        );
        // Modifiers on an arrow: xterm's parameter.
        assert_eq!(
            bytes(Key::Named(Named::ArrowRight), Modifiers::CTRL, None),
            Some(b"\x1b[1;5C".to_vec())
        );
    }

    #[test]
    fn full_screen_programs_get_application_cursor_keys() {
        assert_eq!(
            encode_key(&Key::Named(Named::ArrowUp), Modifiers::empty(), None, true),
            Some(b"\x1bOA".to_vec())
        );
    }

    #[test]
    fn indexed_colors_cover_the_cube_and_the_gray_ramp() {
        assert_eq!(indexed(16, true), Color::from_rgb8(0, 0, 0));
        assert_eq!(indexed(231, true), Color::from_rgb8(255, 255, 255));
        assert_eq!(indexed(196, true), Color::from_rgb8(255, 0, 0));
        assert_eq!(indexed(232, true), Color::from_rgb8(8, 8, 8));
        assert_eq!(indexed(255, true), Color::from_rgb8(238, 238, 238));
    }

    #[test]
    fn the_grid_follows_the_size_of_the_canvas() {
        assert_eq!(grid_size(Size::new(800.0, 300.0)), (16, 100));
        // Never smaller than something that can show a prompt.
        assert_eq!(grid_size(Size::new(1.0, 1.0)), (1, 2));
    }

    #[test]
    fn a_selection_is_in_reading_order() {
        let drag = Drag {
            anchor: Some((3, 5)),
            head: Some((1, 2)),
            dragging: false,
            ..Drag::default()
        };
        assert_eq!(drag.range(), Some(((1, 2), (3, 5))));
        let click = Drag {
            anchor: Some((1, 1)),
            head: Some((1, 1)),
            dragging: false,
            ..Drag::default()
        };
        assert_eq!(click.range(), None);
    }

    #[test]
    fn the_mouse_is_reported_the_way_the_program_asked() {
        let sgr = mouse_report(true, 0, false, (4, 9));
        assert_eq!(sgr, b"\x1b[<0;10;5M");
        let release = mouse_report(true, 0, true, (4, 9));
        assert_eq!(release, b"\x1b[<0;10;5m");
        let old = mouse_report(false, 64, false, (0, 0));
        assert_eq!(old, vec![0x1b, b'[', b'M', 96, 33, 33]);
    }

    #[test]
    fn the_word_under_a_click_loses_the_punctuation_around_it() {
        let row = "error at (src/app.rs:12:5), see";
        assert_eq!(word_at(row, 14).as_deref(), Some("src/app.rs:12:5"));
        assert_eq!(word_at(row, 5), None);
        assert_eq!(word_at("", 0), None);
    }

    #[test]
    fn the_text_of_the_runs_is_put_back_in_its_columns() {
        let grid = Grid {
            id: "t".into(),
            size: (2, 12),
            lines: vec![
                vec![
                    Run {
                        col: 0,
                        text: "ls".into(),
                        ..Run::default()
                    },
                    Run {
                        col: 4,
                        text: "日".into(),
                        wide: true,
                        ..Run::default()
                    },
                    Run {
                        col: 6,
                        text: "a.rs".into(),
                        ..Run::default()
                    },
                ],
                vec![Run {
                    col: 1,
                    text: "ok".into(),
                    ..Run::default()
                }],
            ],
            ..Grid::default()
        };
        let view = ScreenView {
            key: ("p".into(), "t".into()),
            grid: &grid,
            dark: true,
            focused: true,
        };
        assert_eq!(view.text_between(0, 0, 12), "ls  日 a.rs");
        assert_eq!(view.selected_text(((0, 6), (1, 2))), "a.rs\n ok");
        let Message::Plugin(PluginMsg::Ui(_, _, id, Some(value))) =
            view.send(ScreenInput::Resize { rows: 3, cols: 9 })
        else {
            panic!("an event of the screen");
        };
        assert_eq!(id, "t");
        assert_eq!(value["kind"], "resize");
    }
}
