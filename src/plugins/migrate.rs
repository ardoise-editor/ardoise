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

/// The official Spelling plugin.
pub const SPELL: &str = "simpleedit.spell";

/// The words the user added to the dictionary, kept by the editor before
/// the Spelling plugin: copied to its data folder, once.
pub fn carry_personal_dictionary() {
    let old = crate::fsutil::config_dir().map(|d| d.join("simpleedit").join("dictionary.txt"));
    if let (Some(old), Some(dir)) = (old, super::store::data_dir_of(SPELL)) {
        carry_file(&old, &dir.join("dictionary.txt"));
    }
}

/// The official Terminal plugin.
pub const TERMINAL: &str = "simpleedit.terminal";

/// The official Remote Files plugin.
pub const REMOTE: &str = "simpleedit.remote";

/// The machines and the links of the copies of remote files the editor
/// kept before the Remote Files plugin: they become the plugin's, once.
pub fn carry_remote(hosts: &[String], links: &[crate::session::RemoteSession]) {
    if let Some(dir) = super::store::data_dir_of(REMOTE) {
        carry_remote_to(hosts, links, &dir);
    }
}

fn carry_remote_to(hosts: &[String], links: &[crate::session::RemoteSession], dir: &Path) {
    let file = dir.join("remote.json");
    if file.exists() || (hosts.is_empty() && links.is_empty()) {
        return;
    }
    let links: serde_json::Map<String, serde_json::Value> = links
        .iter()
        .map(|l| {
            (
                l.local.clone(),
                serde_json::json!({ "host": l.host, "path": l.path, "date": l.date }),
            )
        })
        .collect();
    let json = serde_json::json!({ "hosts": hosts, "links": links }).to_string();
    let _ = crate::fsutil::write_atomic(&file, json.as_bytes(), false);
}

/// The official Tasks plugin.
pub const TASKS: &str = "simpleedit.tasks";

/// The official Local History plugin.
pub const HISTORY: &str = "simpleedit.history";

/// The history the editor kept before the Local History plugin, and how
/// long it kept it: they become the plugin's, once.
pub fn carry_local_history(config: &crate::config::Config) {
    let (Some(data), Some(dir)) = (
        crate::fsutil::data_dir(),
        super::store::data_dir_of(HISTORY),
    ) else {
        return;
    };
    carry_local_history_to(config, &data.join("simpleedit").join("local-history"), &dir);
}

fn carry_local_history_to(config: &crate::config::Config, old: &Path, dir: &Path) {
    let new = dir.join("local-history");
    if old.is_dir() && !new.exists() {
        let _ = std::fs::rename(old, &new);
    }
    let settings = dir.join("settings.json");
    if !settings.exists() {
        let json = serde_json::json!({ "days": config.local_history_days }).to_string();
        let _ = crate::fsutil::write_atomic(&settings, json.as_bytes(), false);
    }
}

/// Copies `from` to `to` unless `to` is there or `from` is not.
fn carry_file(from: &Path, to: &Path) {
    if to.exists() {
        return;
    }
    if let Ok(bytes) = std::fs::read(from) {
        let _ = crate::fsutil::write_atomic(to, &bytes, false);
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
        // What the plugin changed since is left alone (and a file is copied once).
        let other = dir.path().join("words.txt");
        std::fs::write(&other, "zork\n").unwrap();
        let copy = dir.path().join("copy.txt");
        carry_file(&other, &copy);
        std::fs::write(&other, "new\n").unwrap();
        carry_file(&other, &copy);
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), "zork\n");
        // The history moves to the plugin with how long it is kept.
        let old = dir.path().join("old-history");
        std::fs::create_dir_all(old.join("files")).unwrap();
        let plugin = dir.path().join("plugin");
        std::fs::create_dir_all(&plugin).unwrap();
        config.local_history_days = 30;
        carry_local_history_to(&config, &old, &plugin);
        assert!(plugin.join("local-history/files").is_dir() && !old.exists());
        let days: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(plugin.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(days["days"], 30);
        // The machines and links of the remote files, once.
        let remote = dir.path().join("remote");
        std::fs::create_dir_all(&remote).unwrap();
        let link = crate::session::RemoteSession {
            local: "/l/a.txt".into(),
            host: "me@box".into(),
            path: "/a.txt".into(),
            date: Some(3),
        };
        carry_remote_to(&["me@box".to_string()], &[link], &remote);
        let kept: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(remote.join("remote.json")).unwrap())
                .unwrap();
        assert_eq!(kept["hosts"][0], "me@box");
        assert_eq!(kept["links"]["/l/a.txt"]["date"], 3);
        std::fs::write(&file, "{}").unwrap();
        carry_lint_settings_to(&config, dir.path());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{}");
    }
}
