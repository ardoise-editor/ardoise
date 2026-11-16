//! The places a line of text names (`src/a.rs:12:5`, `File "a.py", line 3`):
//! a click on one in the terminal opens the file there.

use std::path::{Path, PathBuf};

/// A place an output line names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    /// From 1.
    pub line: usize,
    /// From 1; 1 when the line gives none.
    pub col: usize,
}

/// The first place a line of output names that is a file: `path:12:5`,
/// `path:12`, `path(12,5)` (TypeScript), `--> path:12:5` (Rust) or
/// `File "path", line 12` (Python). Relative paths are from `dir`.
#[cfg(test)]
pub fn location(line: &str, dir: &Path) -> Option<Location> {
    location_with(line, dir, &mut |p| p.is_file())
}

/// [`location`], asking `is_file` whether a path is a file (so that a caller
/// reading many lines can remember the answers).
pub fn location_with(
    line: &str,
    dir: &Path,
    is_file: &mut dyn FnMut(&Path) -> bool,
) -> Option<Location> {
    use std::sync::OnceLock;
    static PYTHON: OnceLock<regex::Regex> = OnceLock::new();
    static PLAIN: OnceLock<regex::Regex> = OnceLock::new();
    let python = PYTHON.get_or_init(|| {
        regex::Regex::new(r#"File "([^"]+)", line (\d+)"#).expect("a valid pattern")
    });
    let plain = PLAIN.get_or_init(|| {
        regex::Regex::new(r#"([^\s:()"'<>|]+\.[A-Za-z0-9_]+)(?::|\()(\d+)(?:[:,](\d+))?"#)
            .expect("a valid pattern")
    });
    static SPACED: OnceLock<regex::Regex> = OnceLock::new();
    let spaced = SPACED.get_or_init(|| {
        regex::Regex::new(r#"(/[^:()"'<>|]+\.[A-Za-z0-9_]+)(?::|\()(\d+)(?:[:,](\d+))?"#)
            .expect("a valid pattern")
    });
    let mut resolve = |path: &str| {
        let path = PathBuf::from(path);
        let full = if path.is_absolute() {
            path
        } else {
            dir.join(path)
        };
        is_file(&full).then_some(full)
    };
    if let Some(c) = python.captures(line) {
        if let Some(path) = resolve(&c[1]) {
            return Some(Location {
                path,
                line: c[2].parse().ok()?,
                col: 1,
            });
        }
    }
    for c in plain.captures_iter(line) {
        let Some(path) = resolve(&c[1]) else { continue };
        return Some(Location {
            path,
            line: c[2].parse().ok()?,
            col: c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(1),
        });
    }
    // An absolute path with spaces: the longest tail that is a file.
    for c in spaced.captures_iter(line) {
        let whole = &c[1];
        let starts = whole.match_indices('/').map(|(i, _)| i);
        for start in starts {
            if let Some(path) = resolve(&whole[start..]) {
                return Some(Location {
                    path,
                    line: c[2].parse().ok()?,
                    col: c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(1),
                });
            }
        }
    }
    None
}

/// The byte column of the `col`-th character (1-based, as compilers count)
/// of `line` (1-based) in the file; `col` itself when the file is unreadable.
pub fn byte_col(path: &Path, line: usize, col: usize) -> usize {
    let Ok(text) = std::fs::read_to_string(path) else {
        return col;
    };
    let Some(text) = text.lines().nth(line.saturating_sub(1)) else {
        return col;
    };
    match text.char_indices().nth(col.saturating_sub(1)) {
        Some((i, _)) => i + 1,
        None => text.len() + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_with_spaces_and_a_character_column() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("Mon Projet");
        std::fs::create_dir_all(&sub).unwrap();
        let file = sub.join("a.rs");
        std::fs::write(&file, "é = 1;\n").unwrap();
        let line = format!("--> {}:1:3", file.display());
        let at = location(&line, dir.path()).unwrap();
        assert_eq!(at.path, file);
        assert_eq!(byte_col(&at.path, 1, at.col), 4);
    }

    #[test]
    fn the_places_an_output_names_are_found_when_the_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.rs"), "").unwrap();
        std::fs::write(root.join("b.py"), "").unwrap();
        let at = |line: &str| location(line, root).map(|l| (l.path, l.line, l.col));
        let a = root.join("src/a.rs");
        assert_eq!(at("  --> src/a.rs:12:5"), Some((a.clone(), 12, 5)));
        assert_eq!(at("src/a.rs:7: error: x"), Some((a.clone(), 7, 1)));
        assert_eq!(at("src/a.rs(3,9): error TS1"), Some((a.clone(), 3, 9)));
        assert_eq!(
            at(r#"  File "b.py", line 4, in <module>"#),
            Some((root.join("b.py"), 4, 1))
        );
        // A name that is not a file, or a time, is no place.
        assert_eq!(at("src/none.rs:12:5"), None);
        assert_eq!(at("took 12:30:45"), None);
        assert_eq!(at("nothing here"), None);
    }
}
