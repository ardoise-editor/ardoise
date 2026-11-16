//! Lint: the problems that linters and language servers find in a file,
//! shown in the editor with a fix for those that have one.
//!
//! The linters (eslint, ruff, shellcheck, …) are plugins that declare
//! `contributes.linter` (the official one is `simpleedit.lint`); they answer
//! `lint/run` with [`Diagnostic`]s. Language servers give theirs through
//! `crate::lsp`.

use std::path::Path;

/// How bad a problem is; the order is the order of importance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
}

/// When the linters run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// A moment after the last key typed (the tools that read the disk, on
    /// save).
    #[default]
    OnType,
    OnSave,
    /// Only when asked.
    Manual,
}

impl Trigger {
    pub const ALL: [Trigger; 3] = [Trigger::OnType, Trigger::OnSave, Trigger::Manual];
}

impl std::fmt::Display for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Trigger::OnType => t!("lint.trigger_type"),
            Trigger::OnSave => t!("lint.trigger_save"),
            Trigger::Manual => t!("lint.trigger_manual"),
        })
    }
}

/// What is shown of the problems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Errors,
    #[default]
    Warnings,
    /// Notes and hints too.
    All,
}

impl Level {
    pub const ALL: [Level; 3] = [Level::Errors, Level::Warnings, Level::All];

    pub fn shows(self, severity: Severity) -> bool {
        match self {
            Level::Errors => severity == Severity::Error,
            Level::Warnings => severity >= Severity::Warning,
            Level::All => true,
        }
    }
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Level::Errors => t!("lint.level_errors"),
            Level::Warnings => t!("lint.level_warnings"),
            Level::All => t!("lint.level_all"),
        })
    }
}

/// How a problem is marked in the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Marking {
    /// A wavy line under it.
    Wavy,
    /// A plain line under it.
    Straight,
    /// The text on a colored background.
    Highlight,
    /// A wavy line and a faint background.
    #[default]
    WavyHighlight,
}

impl Marking {
    pub const ALL: [Marking; 4] = [
        Marking::Wavy,
        Marking::Straight,
        Marking::Highlight,
        Marking::WavyHighlight,
    ];
}

impl std::fmt::Display for Marking {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Marking::Wavy => t!("lint.marking_wavy"),
            Marking::Straight => t!("lint.marking_straight"),
            Marking::Highlight => t!("lint.marking_highlight"),
            Marking::WavyHighlight => t!("lint.marking_both"),
        })
    }
}

/// The names the colors of the settings go by.
pub fn color_key(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    }
}

/// How a severity is marked: the one chosen, or the default.
pub fn marking(config: &crate::config::Config, severity: Severity) -> Marking {
    config
        .lint_markings
        .get(color_key(severity))
        .copied()
        .unwrap_or_default()
}

/// `#rrggbb` (or `rrggbb`) as a color.
pub fn parse_hex(text: &str) -> Option<iced::Color> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(iced::Color::from_rgb8(byte(0)?, byte(2)?, byte(4)?))
}

/// The color a severity is shown in: the one chosen, or the default.
pub fn color(config: &crate::config::Config, severity: Severity) -> iced::Color {
    let dark = config.dark_mode;
    config
        .lint_colors
        .get(color_key(severity))
        .and_then(|hex| parse_hex(hex))
        .unwrap_or_else(|| default_color(severity, dark))
}

pub fn default_color(severity: Severity, dark: bool) -> iced::Color {
    match severity {
        Severity::Error => crate::theme::error_color(dark),
        Severity::Warning => iced::Color::from_rgb8(0xe0, 0xa1, 0x1b),
        Severity::Info => iced::Color::from_rgb8(0x4f, 0x9d, 0xe8),
        Severity::Hint => crate::theme::muted_text(dark),
    }
}

/// A change made to the settings of the lint.
#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    Enabled(bool),
    Highlight(bool),
    OnOpen(bool),
    /// How a severity is marked.
    Marking(Severity, Marking),
    FixLevel(Level),
    /// The color of a severity, as typed (empty: the default).
    Color(Severity, String),
    Trigger(Trigger),
    Level(Level),
    FixOnSave(bool),
}

/// The settings with `setting` applied.
pub fn apply_setting(config: &mut crate::config::Config, setting: Setting) {
    match setting {
        Setting::Enabled(on) => config.lint_enabled = on,
        Setting::Highlight(on) => config.lint_highlight = on,
        Setting::OnOpen(on) => config.lint_on_open = on,
        Setting::Marking(severity, m) => {
            config
                .lint_markings
                .insert(color_key(severity).to_string(), m);
        }
        Setting::FixLevel(l) => config.lint_fix_level = l,
        Setting::Color(severity, text) => {
            if text.trim().is_empty() {
                config.lint_colors.remove(color_key(severity));
            } else {
                config
                    .lint_colors
                    .insert(color_key(severity).to_string(), text);
            }
        }
        Setting::Trigger(t) => config.lint_trigger = t,
        Setting::Level(l) => config.lint_level = l,
        Setting::FixOnSave(on) => config.lint_fix_on_save = on,
    }
}

/// A replacement in the text a diagnostic was found in: lines from 0, columns
/// in bytes, the end excluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub text: String,
}

/// What puts a problem right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub title: String,
    pub edits: Vec<Edit>,
    /// Safe to apply without looking (those that may change what the code
    /// does are left to be chosen one by one).
    pub safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub severity: Severity,
    pub message: String,
    /// The rule: `comma-spacing`, `TS2552`, `E501`…
    pub code: Option<String>,
    /// Who found it: `ESLint`, `ruff`, `TypeScript`…
    pub source: String,
    pub fixes: Vec<Fix>,
    /// The problem as a language server said it (to ask it for fixes).
    pub raw: Option<serde_json::Value>,
}

/// How many problems of each severity there are.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub errors: usize,
    pub warnings: usize,
    pub others: usize,
}

#[cfg(test)]
pub fn counts(diagnostics: &[Diagnostic]) -> Counts {
    counts_of(diagnostics.iter())
}

/// The same, for diagnostics that are not in a list of their own (nothing is
/// copied: it is made at each frame).
pub fn counts_of<'a>(diagnostics: impl Iterator<Item = &'a Diagnostic>) -> Counts {
    diagnostics.fold(Counts::default(), |mut c, d| {
        match d.severity {
            Severity::Error => c.errors += 1,
            Severity::Warning => c.warnings += 1,
            _ => c.others += 1,
        }
        c
    })
}

/// The byte offset of (`line`, `col`) in `text`.
pub fn offset_of(text: &str, line: usize, col: usize) -> usize {
    let mut start = 0;
    for _ in 0..line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    // Never inside a character: a position from older text could land there.
    let mut at = (start + col).min(end);
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// `text` with the fixes applied. A fix that touches what another one
/// already changed is left out (it applies on the next run). Returns the new
/// text and how many fixes went in.
pub fn apply(text: &str, fixes: &[&Fix]) -> (String, usize) {
    let mut taken: Vec<(usize, usize, &str)> = Vec::new();
    let mut applied = 0;
    let mut ordered: Vec<&&Fix> = fixes.iter().collect();
    ordered.sort_by_key(|f| {
        f.edits
            .iter()
            .map(|e| offset_of(text, e.line, e.col))
            .min()
            .unwrap_or(usize::MAX)
    });
    for fix in ordered {
        let spans: Vec<(usize, usize, &str)> = fix
            .edits
            .iter()
            .map(|e| {
                let start = offset_of(text, e.line, e.col);
                let end = offset_of(text, e.end_line, e.end_col).max(start);
                (start, end, e.text.as_str())
            })
            .collect();
        let clash = spans.iter().any(|(s, e, _)| {
            taken
                .iter()
                .any(|(ts, te, _)| (s < te && ts < e) || (s == ts && e == te))
        });
        if clash || spans.is_empty() {
            continue;
        }
        taken.extend(spans);
        applied += 1;
    }
    taken.sort_by_key(|(s, e, _)| (*s, *e));
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (start, end, replacement) in taken {
        if start < at {
            continue;
        }
        out.push_str(&text[at..start]);
        out.push_str(replacement);
        at = end;
    }
    out.push_str(&text[at..]);
    (out, applied)
}

/// The same, for the problems that `level` shows.
pub fn fixes_at(diagnostics: &[Diagnostic], level: Level) -> Vec<&Fix> {
    diagnostics
        .iter()
        .filter(|d| level.shows(d.severity))
        .filter_map(|d| d.fixes.first().filter(|f| f.safe))
        .collect()
}

// ─── What the linters of the plugins say ─────────────────────────────────

use simpleedit_plugin::lint as from;

impl From<from::Severity> for Severity {
    fn from(s: from::Severity) -> Self {
        match s {
            from::Severity::Hint => Severity::Hint,
            from::Severity::Info => Severity::Info,
            from::Severity::Warning => Severity::Warning,
            from::Severity::Error => Severity::Error,
        }
    }
}

impl From<Level> for from::Level {
    fn from(l: Level) -> Self {
        match l {
            Level::Errors => from::Level::Errors,
            Level::Warnings => from::Level::Warnings,
            Level::All => from::Level::All,
        }
    }
}

impl From<from::Fix> for Fix {
    fn from(f: from::Fix) -> Self {
        Fix {
            title: f.title,
            safe: f.safe,
            edits: f
                .edits
                .into_iter()
                .map(|e| Edit {
                    line: e.line,
                    col: e.col,
                    end_line: e.end_line,
                    end_col: e.end_col,
                    text: e.text,
                })
                .collect(),
        }
    }
}

impl From<from::Diagnostic> for Diagnostic {
    fn from(d: from::Diagnostic) -> Self {
        Diagnostic {
            line: d.line,
            col: d.col,
            end_line: d.end_line.max(d.line),
            end_col: d.end_col,
            severity: d.severity.into(),
            message: d.message,
            code: d.code,
            source: d.source,
            fixes: d.fixes.into_iter().map(Fix::from).collect(),
            raw: None,
        }
    }
}

/// Whether a language server may give the problems of this extension.
pub fn server_ready(ext: &str) -> bool {
    let ext = ext.to_ascii_lowercase();
    crate::lsp::spec_for(&ext).is_some() && crate::lsp::installed(&ext)
}

/// `text` with the safe fixes the language server has for the problems
/// that `level` covers (of `source` only, if given): one per problem.
pub async fn lsp_fix(
    ext: &str,
    path: &Path,
    text: String,
    level: Level,
    source: Option<&str>,
) -> String {
    let Ok(problems) = crate::lsp::diagnostics(ext, path, &text).await else {
        return text;
    };
    let wanted: Vec<_> = problems
        .iter()
        .filter(|d| level.shows(d.severity) && source.is_none_or(|s| s == d.source))
        .take(300)
        .collect();
    let fixes: Vec<_> = crate::lsp::first_fixes(ext, path, &text, &wanted)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|f| f.safe)
        .collect();
    let refs: Vec<&Fix> = fixes.iter().collect();
    apply(&text, &refs).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_problems_of_a_plugin_keep_their_place_rule_and_fixes() {
        let d = from::Diagnostic {
            line: 2,
            col: 1,
            end_line: 2,
            end_col: 4,
            severity: from::Severity::Warning,
            message: "m".into(),
            code: Some("E1".into()),
            source: "ruff".into(),
            fixes: vec![from::Fix {
                title: "t".into(),
                edits: vec![from::Edit {
                    line: 2,
                    col: 1,
                    end_line: 2,
                    end_col: 4,
                    text: "x".into(),
                }],
                safe: true,
            }],
        };
        let ours = Diagnostic::from(d);
        assert_eq!((ours.line, ours.col, ours.end_col), (2, 1, 4));
        assert_eq!(ours.severity, Severity::Warning);
        assert_eq!(ours.code.as_deref(), Some("E1"));
        assert!(ours.fixes[0].safe && ours.fixes[0].edits[0].text == "x");
        assert_eq!(from::Level::from(Level::Errors), from::Level::Errors);
    }

    #[test]
    fn settings_change_the_config() {
        let mut config = crate::config::Config::default();
        apply_setting(&mut config, Setting::FixOnSave(true));
        assert!(config.lint_fix_on_save);
        apply_setting(&mut config, Setting::Enabled(false));
        apply_setting(&mut config, Setting::Level(Level::Errors));
        assert!(!config.lint_enabled && config.lint_level == Level::Errors);
    }

    #[test]
    fn colors_are_the_ones_chosen_or_the_defaults() {
        assert_eq!(
            parse_hex("#ff8000"),
            Some(iced::Color::from_rgb8(255, 128, 0))
        );
        assert_eq!(parse_hex("00ff00"), Some(iced::Color::from_rgb8(0, 255, 0)));
        assert!(parse_hex("#ff80").is_none() && parse_hex("zzzzzz").is_none());
        let mut config = crate::config::Config::default();
        let default = color(&config, Severity::Warning);
        assert_eq!(default, default_color(Severity::Warning, true));
        apply_setting(
            &mut config,
            Setting::Color(Severity::Warning, "#112233".into()),
        );
        assert_eq!(
            color(&config, Severity::Warning),
            iced::Color::from_rgb8(0x11, 0x22, 0x33)
        );
        // Half typed: the default stays until it is a color.
        apply_setting(
            &mut config,
            Setting::Color(Severity::Warning, "#1122".into()),
        );
        assert_eq!(color(&config, Severity::Warning), default);
        apply_setting(&mut config, Setting::Color(Severity::Warning, " ".into()));
        assert!(config.lint_colors.is_empty());
        // Each severity has its own way of being marked.
        assert_eq!(marking(&config, Severity::Error), Marking::WavyHighlight);
        apply_setting(
            &mut config,
            Setting::Marking(Severity::Error, Marking::Highlight),
        );
        apply_setting(
            &mut config,
            Setting::Marking(Severity::Warning, Marking::Straight),
        );
        assert_eq!(marking(&config, Severity::Error), Marking::Highlight);
        assert_eq!(marking(&config, Severity::Warning), Marking::Straight);
        assert_eq!(marking(&config, Severity::Info), Marking::WavyHighlight);
    }

    #[test]
    fn fixing_them_all_goes_by_the_level() {
        let fix = |text: &str| Fix {
            title: String::new(),
            safe: true,
            edits: vec![Edit {
                line: 0,
                col: 0,
                end_line: 0,
                end_col: 0,
                text: text.into(),
            }],
        };
        let d = |severity, text: &str| Diagnostic {
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 1,
            severity,
            message: String::new(),
            code: None,
            source: String::new(),
            fixes: vec![fix(text)],
            raw: None,
        };
        let all = [
            d(Severity::Error, "e"),
            d(Severity::Warning, "w"),
            d(Severity::Hint, "h"),
        ];
        assert_eq!(fixes_at(&all, Level::Errors).len(), 1);
        assert_eq!(fixes_at(&all, Level::Warnings).len(), 2);
        assert_eq!(fixes_at(&all, Level::All).len(), 3);
        assert_eq!(
            crate::config::Config::default().lint_fix_level,
            Level::Errors
        );
    }

    #[test]
    fn the_level_picks_what_is_shown() {
        assert!(Level::Errors.shows(Severity::Error) && !Level::Errors.shows(Severity::Warning));
        assert!(Level::Warnings.shows(Severity::Warning) && !Level::Warnings.shows(Severity::Info));
        assert!(Level::All.shows(Severity::Hint));
    }

    fn edit(line: usize, col: usize, end_col: usize, text: &str) -> Edit {
        Edit {
            line,
            col,
            end_line: line,
            end_col,
            text: text.into(),
        }
    }

    fn fix(edits: Vec<Edit>) -> Fix {
        Fix {
            title: "fix".into(),
            edits,
            safe: true,
        }
    }

    #[test]
    fn offsets_are_found_in_the_lines() {
        let text = "ab\ncde\n\nf";
        assert_eq!(offset_of(text, 1, 2), 5);
        assert_eq!(offset_of(text, 3, 0), 8);
        assert_eq!(offset_of(text, 9, 0), text.len());
        // A column past the end of a line stays on the line.
        assert_eq!(offset_of(text, 0, 50), 2);
    }

    #[test]
    fn fixes_apply_together_and_a_clash_waits() {
        let text = "a ,b ,c\n";
        let one = fix(vec![edit(0, 1, 2, "")]);
        let two = fix(vec![edit(0, 4, 5, "")]);
        let (out, n) = apply(text, &[&two, &one]);
        assert_eq!((out.as_str(), n), ("a,b,c\n", 2));
        // The same text changed twice: the second is left for later.
        let clash = fix(vec![edit(0, 1, 3, "X")]);
        let (out, n) = apply(text, &[&one, &clash]);
        assert_eq!((out.as_str(), n), ("a,b ,c\n", 1));
        // An insertion, and a fix with several edits.
        let both = fix(vec![edit(0, 0, 0, "["), edit(0, 7, 7, "]")]);
        assert_eq!(apply(text, &[&both]).0, "[a ,b ,c]\n");
    }

    #[test]
    fn problems_are_counted_by_severity() {
        let d = |severity| Diagnostic {
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 1,
            severity,
            message: String::new(),
            code: None,
            source: String::new(),
            fixes: vec![],
            raw: None,
        };
        let c = counts(&[
            d(Severity::Error),
            d(Severity::Warning),
            d(Severity::Warning),
            d(Severity::Hint),
        ]);
        assert_eq!((c.errors, c.warnings, c.others), (1, 2, 1));
    }
}
