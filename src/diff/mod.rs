//! Diffs and how files stand: what a diff compares, the changes it holds
//! (parsed from the text `git diff` writes), the conflicts of a file, the
//! lines changed since a text.

pub mod conflicts;
pub mod marks;
pub mod parse;

pub use parse::FileDiff;

/// How a file stands (as a plugin says), as the tree of projects colors it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Modified,
    Added,
    Untracked,
    Conflicted,
    Ignored,
}

/// A file a diff is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub path: String,
    /// Where a renamed file came from.
    pub from: Option<String>,
}

/// What a diff compares: it decides the buttons of its blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Only to read.
    Plain,
    /// Changes not staged: each block can be staged, or put back.
    Unstaged,
    /// Changes staged: each block can be unstaged, or put back.
    Staged,
    /// What a revision changed; it can be put against the file now.
    Commit(String),
    /// A revision against the file now: each block can be put back; it can
    /// be turned back into the revision's own diff.
    Tree(String),
}
