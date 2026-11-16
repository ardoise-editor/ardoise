//! What users of the versions before the plugins had set, carried over to
//! the plugins that took over the tools.

use std::path::Path;

/// The official Lint plugin.
pub const LINT: &str = "simpleedit.lint";

/// The languages, tools and programs of the lint, as the Lint plugin keeps
/// them (`settings.json` in its data folder). Written once: a file that is
/// there is the plugin's own.
pub fn carry_lint_settings(config: &crate::config::Config) {
    if let Some(dir) = super::store::data_dir_of(LINT) {
        carry_lint_settings_to(config, &dir);
    }
}

fn carry_lint_settings_to(config: &crate::config::Config, dir: &Path) {
    let file = dir.join("settings.json");
    let used = !config.lint_languages.is_empty()
        || !config.lint_tools.is_empty()
        || !config.lint_programs.is_empty();
    if !used || file.exists() {
        return;
    }
    let settings = serde_json::json!({
        "languages": config.lint_languages,
        "tools": config.lint_tools,
        "programs": config.lint_programs,
    });
    if let Ok(json) = serde_json::to_string_pretty(&settings) {
        let _ = crate::fsutil::write_atomic(&file, json.as_bytes(), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lint_settings_go_to_the_plugin_once() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        // Nothing was set: the plugin starts from its own defaults.
        let mut config = crate::config::Config::default();
        carry_lint_settings_to(&config, dir.path());
        assert!(!file.exists());
        config.lint_languages.insert("py".into());
        config
            .lint_tools
            .insert("py".into(), vec!["ruff".into(), "lsp".into()]);
        config
            .lint_programs
            .insert("ruff".into(), "/opt/ruff".into());
        carry_lint_settings_to(&config, dir.path());
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(written["languages"], serde_json::json!(["py"]));
        assert_eq!(written["tools"]["py"][1], "lsp");
        assert_eq!(written["programs"]["ruff"], "/opt/ruff");
        // What the plugin changed since is left alone.
        std::fs::write(&file, "{}").unwrap();
        carry_lint_settings_to(&config, dir.path());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{}");
    }
}
