//! A diff on screen: the files it walks through, the one shown, the place
//! in it and what the mouse picked. Diff tabs and comparisons use it.

pub mod merge;
pub mod syntax;
pub mod view;

use iced::Task;

use crate::app::Message;
use crate::diff::{Change, FileDiff, Source};

/// A file of the list the viewer walks through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub source: Source,
    pub change: Change,
    /// In the index (the "staged" part of the commit list).
    pub staged: bool,
}

/// Where to land in a file once its diff has loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Land {
    First,
    Last,
}

/// The diff pane: a list of files, one open, and a place in it.
pub struct Viewer {
    pub entries: Vec<Entry>,
    pub index: Option<usize>,
    pub diff: Option<Result<FileDiff, String>>,
    /// The change block the arrows are on.
    pub change: usize,
    pub side_by_side: bool,
    /// Bumped each time `diff` really changes: the syntax colors follow it,
    /// not the request token (a quiet look that finds nothing new keeps them).
    diff_rev: u64,
    /// What drawing the diff needs that costs a walk through its text.
    layout: view::LayoutCache,
    /// Where its diff scrolls.
    pub scroll: iced::widget::Id,
    /// How many characters a side of the side-by-side view holds on a line,
    /// as the view last saw it (0: not known yet); long lines wrap there.
    pub columns: std::cell::Cell<usize>,
    /// How far down the diff is scrolled and how tall it shows: only the rows
    /// around that are built.
    pub view_top: std::cell::Cell<f32>,
    pub view_height: std::cell::Cell<f32>,
    /// The syntax colors of the open diff (for the theme they were made for,
    /// and the request they belong to).
    syntax: std::cell::RefCell<Option<(bool, u64, std::sync::Arc<syntax::Spans>)>>,
    /// The rows picked with the mouse (to copy them).
    pub selection: Option<Selection>,
    /// How the diff is read: changes of blanks left out, lines of context.
    pub ignore_blanks: bool,
    pub context: u32,
    /// The mouse button is held down over the diff.
    dragging: bool,
    /// The last press, to tell a double click.
    last_press: Option<(std::time::Instant, Pos)>,
    /// The diff of each file (one per entry).
    pub provided: Option<Vec<FileDiff>>,
}

/// A place in the diff: a row and a character of its text (tabs counted as
/// four spaces, as shown).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

/// Text of the diff picked with the mouse, from `anchor` to `cursor`; in the
/// side-by-side view, within one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Pos,
    pub cursor: Pos,
    pub right: bool,
}

impl Selection {
    fn ends(&self) -> (Pos, Pos) {
        (self.anchor.min(self.cursor), self.anchor.max(self.cursor))
    }

    /// The characters of a row of `len` that are picked.
    pub fn span(&self, row: usize, len: usize) -> Option<(usize, usize)> {
        let (lo, hi) = self.ends();
        if row < lo.row || row > hi.row {
            return None;
        }
        let start = if row == lo.row { lo.col.min(len) } else { 0 };
        let end = if row == hi.row { hi.col.min(len) } else { len };
        (start < end).then_some((start, end))
    }
}

/// What the mouse does on the text of a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffPointer {
    /// Pressed on a character of a row (side by side: of the right half or not).
    Press(usize, bool, usize),
    /// Dragged to a character of a row.
    Drag(usize, usize),
    Release,
}

/// The word around a character of a text (or the run of blanks, or the one
/// symbol), as the characters it spans.
fn word_at(text: &str, col: usize) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return (0, 0);
    }
    let at = col.min(chars.len() - 1);
    let class = |c: char| {
        if c.is_alphanumeric() || c == '_' {
            0
        } else if c.is_whitespace() {
            1
        } else {
            2
        }
    };
    let kind = class(chars[at]);
    if kind == 2 {
        return (at, at + 1);
    }
    let mut start = at;
    while start > 0 && class(chars[start - 1]) == kind {
        start -= 1;
    }
    let mut end = at + 1;
    while end < chars.len() && class(chars[end]) == kind {
        end += 1;
    }
    (start, end)
}

/// The part of a text between two columns of its shown form, where a tab
/// counts as four.
fn original_slice(text: &str, start: usize, end: usize) -> String {
    let mut shown = 0;
    let mut out = String::new();
    for c in text.chars() {
        let width = if c == '\t' { 4 } else { 1 };
        if shown >= start && shown < end {
            out.push(c);
        }
        shown += width;
    }
    out
}

impl Default for Viewer {
    fn default() -> Self {
        Viewer {
            entries: Vec::new(),
            index: None,
            diff: None,
            change: 0,
            side_by_side: true,
            diff_rev: 0,
            layout: std::cell::RefCell::new(None),
            scroll: iced::widget::Id::unique(),
            columns: std::cell::Cell::new(0),
            view_top: std::cell::Cell::new(0.0),
            // Until the scrollable says, a tall screen's worth.
            view_height: std::cell::Cell::new(1400.0),
            syntax: std::cell::RefCell::new(None),
            selection: None,
            ignore_blanks: false,
            context: 3,
            dragging: false,
            last_press: None,
            provided: None,
        }
    }
}

impl Viewer {
    /// The syntax colors of the open diff, made when first asked for.
    pub fn syntax_spans(&self, dark: bool) -> Option<std::sync::Arc<syntax::Spans>> {
        let diff = self.file_diff()?;
        let path = &self.current()?.change.path;
        let mut cache = self.syntax.borrow_mut();
        match &*cache {
            Some((was_dark, token, spans)) if *was_dark == dark && *token == self.diff_rev => {
                return Some(spans.clone());
            }
            _ => {}
        }
        let spans = std::sync::Arc::new(syntax::highlight(diff, path, dark));
        *cache = Some((dark, self.diff_rev, spans.clone()));
        Some(spans)
    }

    pub fn diff_rev(&self) -> u64 {
        self.diff_rev
    }

    /// 3 lines of context, then 10, 30, the whole file, and back.
    pub fn cycle_context(&mut self) {
        self.context = match self.context {
            3 => 10,
            10 => 30,
            30 => u32::MAX,
            _ => 3,
        };
    }

    /// The changed lines of the rows picked, by their place in the diff.
    pub fn picked_changes(&self) -> std::collections::HashSet<(usize, usize)> {
        let Some(sel) = self.selection else {
            return Default::default();
        };
        let (lo, hi) = sel.ends();
        self.row_lines(sel.right)
            .into_iter()
            .enumerate()
            .filter(|(row, _)| (lo.row..=hi.row).contains(row))
            .filter_map(|(_, line)| line.filter(|l| l.tag != crate::diff::parse::Tag::Context))
            .map(|l| l.pos)
            .collect()
    }

    /// What staging (or unstaging) a block or lines of the open diff takes:
    /// the patch and which way it goes, when this diff can do it.
    pub fn index_patch(&self, block: Option<usize>) -> Option<(String, bool)> {
        use crate::diff::Source;
        // Blanks left out or a file that is new: the patch would not fit.
        if self.ignore_blanks {
            return None;
        }
        let reverse = match self.current()?.source {
            Source::Unstaged => false,
            Source::Staged => true,
            _ => return None,
        };
        let diff = self.file_diff()?;
        let patch = match block {
            Some(b) => diff.block_patch(b, reverse)?,
            None => diff.lines_patch(&self.picked_changes(), reverse)?,
        };
        Some((patch, reverse))
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn pick(&mut self, pick: DiffPointer) {
        match pick {
            DiffPointer::Press(row, right, col) => {
                let here = Pos { row, col };
                let double = self.last_press.is_some_and(|(at, p)| {
                    at.elapsed() < std::time::Duration::from_millis(400)
                        && p.row == row
                        && p.col.abs_diff(col) <= 1
                });
                self.last_press = Some((std::time::Instant::now(), here));
                self.dragging = true;
                let (anchor, cursor) = match (double, self.line_text(row, right)) {
                    (true, Some(text)) => {
                        let (a, b) = word_at(&text, col);
                        (Pos { row, col: a }, Pos { row, col: b })
                    }
                    _ => (here, here),
                };
                self.selection = Some(Selection {
                    anchor,
                    cursor,
                    right,
                });
            }
            DiffPointer::Drag(row, col) => {
                if let (true, Some(sel)) = (self.dragging, self.selection.as_mut()) {
                    sel.cursor = Pos { row, col };
                }
            }
            DiffPointer::Release => self.dragging = false,
        }
    }

    /// The line each row of the layout in use holds (of one side, side by
    /// side), `None` for a header or an empty half. Built once per use.
    fn row_lines(&self, right: bool) -> Vec<Option<&crate::diff::parse::Line>> {
        let Some(diff) = self.file_diff() else {
            return Vec::new();
        };
        if self.side_by_side {
            diff.side_rows()
                .into_iter()
                .map(|row| match row {
                    crate::diff::parse::SideRow::Pair(left, r) => {
                        if right {
                            r
                        } else {
                            left
                        }
                    }
                    crate::diff::parse::SideRow::Hunk(_) => None,
                })
                .collect()
        } else {
            diff.rows()
                .into_iter()
                .map(|row| match row {
                    crate::diff::parse::Row::Line(l) => Some(l),
                    crate::diff::parse::Row::Hunk(_) => None,
                })
                .collect()
        }
    }

    /// The text of a row as shown (tabs expanded), if it has one.
    fn line_text(&self, row: usize, right: bool) -> Option<String> {
        let lines = self.row_lines(right);
        let line = lines.get(row).copied().flatten()?;
        Some(line.text.replace('\t', "    "))
    }

    /// Every row (of the right side, or of the side last picked).
    pub fn select_all(&mut self) {
        let right = self.selection.is_none_or(|s| s.right);
        let lines = self.row_lines(right);
        let Some(last) = lines.len().checked_sub(1) else {
            return;
        };
        let col = lines[last].map_or(0, |l| l.text.replace('\t', "    ").chars().count());
        self.selection = Some(Selection {
            anchor: Pos { row: 0, col: 0 },
            cursor: Pos { row: last, col },
            right,
        });
    }

    /// Whether something is picked (a click alone picks nothing).
    pub fn has_selection(&self) -> bool {
        self.selection.is_some_and(|s| s.anchor != s.cursor)
    }

    /// The picked text, as it is in the file (tabs back).
    pub fn selected_text(&self) -> Option<String> {
        let sel = self.selection?;
        let (lo, hi) = sel.ends();
        if lo == hi {
            return None;
        }
        let lines = self.row_lines(sel.right);
        let mut out: Vec<String> = Vec::new();
        for row in lo.row..=hi.row {
            let Some(line) = lines.get(row).copied().flatten() else {
                continue;
            };
            let shown = line.text.replace('\t', "    ");
            let Some((start, end)) = sel.span(row, shown.chars().count()) else {
                // A picked line with nothing in it, between the ends.
                if row > lo.row && row < hi.row {
                    out.push(String::new());
                }
                continue;
            };
            out.push(original_slice(&line.text, start, end));
        }
        if out.is_empty() {
            None
        } else {
            Some(out.join("\n"))
        }
    }

    pub fn current(&self) -> Option<&Entry> {
        self.index.and_then(|i| self.entries.get(i))
    }

    pub fn file_diff(&self) -> Option<&FileDiff> {
        match &self.diff {
            Some(Ok(diff)) => Some(diff),
            _ => None,
        }
    }

    pub fn changes(&self) -> usize {
        self.file_diff().map_or(0, FileDiff::changes)
    }

    /// How far down a row starts: long lines of the side-by-side view wrap,
    /// so rows are not all as tall.
    fn row_top(&self, row: usize) -> f32 {
        match (self.side_by_side, self.file_diff()) {
            (true, Some(diff)) => {
                let layout =
                    view::layout(&self.layout, self.diff_rev, diff, self.columns.get(), true);
                view::side_top(&layout, row)
            }
            _ => row as f32 * view::ROW_HEIGHT,
        }
    }

    /// The row to bring into view for the current change.
    pub fn change_row(&self) -> Option<usize> {
        let diff = self.file_diff()?;
        let rows = if self.side_by_side {
            diff.change_side_rows()
        } else {
            diff.change_rows()
        };
        rows.get(self.change).copied()
    }

    /// Shows a file of the list, at its first change or its last.
    pub fn open(&mut self, index: usize, land: Land) -> Task<Message> {
        if index >= self.entries.len() {
            return Task::none();
        }
        self.index = Some(index);
        self.diff = self
            .provided
            .as_ref()
            .and_then(|p| p.get(index))
            .cloned()
            .map(Ok);
        self.diff_rev += 1;
        self.selection = None;
        self.change = match land {
            Land::First => 0,
            Land::Last => self.changes().saturating_sub(1),
        };
        self.view_top.set(0.0);
        self.scroll_to_change()
    }

    /// Shows one diff, beginning at its start.
    pub fn show(&mut self, entry: Entry, diff: FileDiff) {
        self.entries = vec![entry];
        self.index = Some(0);
        self.diff = Some(Ok(diff));
        self.diff_rev += 1;
        self.change = 0;
        self.selection = None;
        self.view_top.set(0.0);
    }

    pub fn scroll_to_change(&self) -> Task<Message> {
        let y = match self.change_row() {
            Some(row) => self.row_top(row.saturating_sub(3)).max(0.0),
            None => 0.0,
        };
        // The rows there are built right away, not when the scrollable says.
        self.view_top.set(y);
        iced::widget::operation::scroll_to(
            self.scroll.clone(),
            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y },
        )
    }

    /// Down: the next change, or the first of the next file.
    pub fn next_change(&mut self) -> Task<Message> {
        if self.change + 1 < self.changes() {
            self.change += 1;
            return self.scroll_to_change();
        }
        // The last one of all: it is brought back into view (with a whole
        // file shown, it may be far from where the eye is).
        if self.index.is_some_and(|i| i + 1 >= self.entries.len()) {
            return self.scroll_to_change();
        }
        self.next_file(Land::First)
    }

    /// Up: the previous change, or the last of the previous file.
    pub fn prev_change(&mut self) -> Task<Message> {
        if self.change > 0 {
            self.change -= 1;
            return self.scroll_to_change();
        }
        if self.index == Some(0) {
            return self.scroll_to_change();
        }
        self.prev_file(Land::Last)
    }

    pub fn next_file(&mut self, land: Land) -> Task<Message> {
        match self.index {
            Some(i) if i + 1 < self.entries.len() => self.open(i + 1, land),
            _ => Task::none(),
        }
    }

    pub fn prev_file(&mut self, land: Land) -> Task<Message> {
        match self.index {
            Some(i) if i > 0 => self.open(i - 1, land),
            _ => Task::none(),
        }
    }
}
