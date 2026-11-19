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

/// The official API plugin.
pub const API: &str = "simpleedit.api";

/// The collections, environments and history of the API client, and what
/// the client trusts and goes through: they become the API plugin's, once.
pub fn carry_api(config: &crate::config::Config) {
    let (Some(data), Some(dir)) = (crate::fsutil::data_dir(), super::store::data_dir_of(API))
    else {
        return;
    };
    carry_api_to(config, &data.join("simpleedit").join("api"), &dir);
}

fn carry_api_to(config: &crate::config::Config, old: &Path, dir: &Path) {
    let new = dir.join("api");
    if old.is_dir() && !new.exists() {
        let _ = std::fs::rename(old, &new);
    }
    let settings = dir.join("settings.json");
    let used = !config.api_ca_file.is_empty()
        || !config.api_client_cert.is_empty()
        || !config.api_proxy.is_empty();
    if used && !settings.exists() {
        let json = serde_json::json!({
            "ca_file": config.api_ca_file,
            "client_cert": config.api_client_cert,
            "proxy": config.api_proxy,
        })
        .to_string();
        let _ = crate::fsutil::write_atomic(&settings, json.as_bytes(), false);
    }
}

/// The API requests changed and not kept, of before the API plugin: added
/// to the tabs it keeps. Their tabs and titles.
pub fn carry_api_tabs(saved: &[crate::session::ApiSession]) -> Vec<(String, String)> {
    match super::store::data_dir_of(API) {
        Some(dir) => carry_api_tabs_to(saved, &dir),
        None => Vec::new(),
    }
}

fn carry_api_tabs_to(saved: &[crate::session::ApiSession], dir: &Path) -> Vec<(String, String)> {
    let file = dir.join("tabs.json");
    let mut kept: serde_json::Value = std::fs::read_to_string(&file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({ "tabs": {}, "counter": 0 }));
    let mut counter = kept["counter"].as_u64().unwrap_or(0);
    let mut opened = Vec::new();
    for s in saved {
        counter += 1;
        let tab = format!("req-{counter}");
        let name = s.request["name"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string();
        let title = if name.is_empty() {
            s.request["url"].as_str().unwrap_or("request").to_string()
        } else {
            name
        };
        kept["tabs"][&tab] = serde_json::json!({
            "request": s.request,
            "kept": s.kept,
            "origin": s.origin,
        });
        opened.push((tab, title));
    }
    kept["counter"] = serde_json::json!(counter);
    if let Ok(json) = serde_json::to_string_pretty(&kept) {
        let _ = crate::fsutil::write_atomic(&file, json.as_bytes(), true);
    }
    opened
}

/// The official Database plugin.
pub const DATABASE: &str = "simpleedit.database";

/// The data sources the editor kept before the Database plugin, and the
/// queries of its console: they become the plugin's, once (the passwords
/// stay in the keychain, under the same names).
pub fn carry_database() {
    let (Some(config), Some(data), Some(dir)) = (
        crate::fsutil::config_dir(),
        crate::fsutil::data_dir(),
        super::store::data_dir_of(DATABASE),
    ) else {
        return;
    };
    carry_database_to(&config.join("simpleedit"), &data.join("simpleedit"), &dir);
}

fn carry_database_to(config: &Path, data: &Path, dir: &Path) {
    carry_file(&config.join("databases.json"), &dir.join("databases.json"));
    carry_file(&data.join("queries.sql"), &dir.join("queries.sql"));
}

/// The official Git plugin.
pub const GIT: &str = "simpleedit.git";

/// How pull and merge were made, and how often to fetch, before Git became
/// the Git plugin: they become its settings, once.
pub fn carry_git_settings(config: &crate::config::Config) {
    if let Some(dir) = super::store::data_dir_of(GIT) {
        carry_git_settings_to(config, &dir);
    }
}

fn carry_git_settings_to(config: &crate::config::Config, dir: &Path) {
    let file = dir.join("settings.json");
    if file.exists() {
        return;
    }
    let pull = match config.git_pull_mode.as_str() {
        "rebase" => "rebase",
        "fast-forward-only" => "ff-only",
        _ => "merge",
    };
    let merge = match config.git_merge_mode.as_str() {
        "no-fast-forward" => "no-ff",
        "fast-forward-only" => "ff-only",
        "squash" => "squash",
        _ => "default",
    };
    let settings = serde_json::json!({
        "pull_mode": pull,
        "merge_mode": merge,
        "auto_fetch_minutes": config.git_auto_fetch_minutes,
    });
    let _ = crate::fsutil::write_atomic(&file, settings.to_string().as_bytes(), false);
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

/// The official plugins a user of a version before the plugins relied on:
/// the tools that were always there, and those they had set up.
pub fn relied_on(config: &crate::config::Config) -> Vec<&'static str> {
    let (config_dir, data) = (
        crate::fsutil::config_dir().map(|d| d.join("simpleedit")),
        crate::fsutil::data_dir().map(|d| d.join("simpleedit")),
    );
    let api = data.as_ref().is_some_and(|d| d.join("api").exists());
    let database = config_dir
        .as_ref()
        .is_some_and(|d| d.join("databases.json").exists());
    relied_on_with(config, api, database)
}

fn relied_on_with(config: &crate::config::Config, api: bool, database: bool) -> Vec<&'static str> {
    let mut wanted = vec![GIT, TERMINAL, TASKS];
    if config.lint_enabled {
        wanted.push(LINT);
    }
    if config.spell_check {
        wanted.push(SPELL);
    }
    if config.local_history_days > 0 {
        wanted.push(HISTORY);
    }
    if !config.remote_hosts.is_empty() {
        wanted.push(REMOTE);
    }
    if api || config.show_api_panel {
        wanted.push(API);
    }
    if database || config.show_database_panel {
        wanted.push(DATABASE);
    }
    wanted
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
        // The API: its folder, its network settings, its tabs not kept.
        let old_api = dir.path().join("old-api");
        std::fs::create_dir_all(old_api.join("collections")).unwrap();
        let api = dir.path().join("api-plugin");
        std::fs::create_dir_all(&api).unwrap();
        config.api_proxy = "http://p:3128".into();
        carry_api_to(&config, &old_api, &api);
        assert!(api.join("api/collections").is_dir());
        let net: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(api.join("settings.json")).unwrap())
                .unwrap();
        assert_eq!(net["proxy"], "http://p:3128");
        let saved = crate::session::ApiSession {
            request: serde_json::json!({"name": "", "url": "https://x/y", "method": "Get"}),
            kept: serde_json::json!({"name": "", "url": ""}),
            origin: None,
        };
        let opened = carry_api_tabs_to(std::slice::from_ref(&saved), &api);
        assert_eq!(
            opened,
            vec![("req-1".to_string(), "https://x/y".to_string())]
        );
        let again = carry_api_tabs_to(&[saved], &api);
        assert_eq!(again[0].0, "req-2", "added after what is kept");
        let tabs: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(api.join("tabs.json")).unwrap()).unwrap();
        assert_eq!(tabs["tabs"]["req-1"]["request"]["url"], "https://x/y");
        std::fs::write(&file, "{}").unwrap();
        carry_lint_settings_to(&config, dir.path());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{}");
    }

    #[test]
    fn the_ways_to_pull_and_merge_and_the_fetches_become_the_git_plugins_once() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::config::Config {
            git_pull_mode: "fast-forward-only".into(),
            git_merge_mode: "no-fast-forward".into(),
            git_auto_fetch_minutes: 10,
            ..Default::default()
        };
        carry_git_settings_to(&config, dir.path());
        let kept: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("settings.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(kept["pull_mode"], "ff-only");
        assert_eq!(kept["merge_mode"], "no-ff");
        assert_eq!(kept["auto_fetch_minutes"], 10);
        // Once: the plugin's own settings stay.
        config.git_pull_mode = "rebase".into();
        carry_git_settings_to(&config, dir.path());
        let kept = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        assert!(kept.contains("ff-only"));
    }

    #[test]
    fn the_data_sources_and_the_queries_become_the_database_plugins_once() {
        let old = tempfile::tempdir().unwrap();
        let (config, data) = (old.path().join("config"), old.path().join("data"));
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(config.join("databases.json"), r#"{"connections": []}"#).unwrap();
        std::fs::write(data.join("queries.sql"), "SELECT 1;").unwrap();
        let dir = tempfile::tempdir().unwrap();
        carry_database_to(&config, &data, dir.path());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("queries.sql")).unwrap(),
            "SELECT 1;"
        );
        assert!(dir.path().join("databases.json").exists());
        // Once: what the plugin has stays.
        std::fs::write(data.join("queries.sql"), "SELECT 2;").unwrap();
        carry_database_to(&config, &data, dir.path());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("queries.sql")).unwrap(),
            "SELECT 1;"
        );
    }

    #[test]
    fn a_user_before_the_plugins_keeps_the_tools_used_and_a_new_install_has_none() {
        let mut config = crate::config::Config {
            lint_enabled: true,
            spell_check: false,
            local_history_days: 5,
            ..Default::default()
        };
        let wanted = relied_on_with(&config, false, true);
        for id in [GIT, TERMINAL, TASKS, LINT, HISTORY, DATABASE] {
            assert!(wanted.contains(&id), "{id}");
        }
        for id in [SPELL, REMOTE, API] {
            assert!(!wanted.contains(&id), "{id}");
        }
        config.remote_hosts.push("server".into());
        assert!(relied_on_with(&config, true, false).contains(&REMOTE));
        assert!(relied_on_with(&config, true, false).contains(&API));
        // A new install: nothing to carry over; a file of a version before
        // the plugins: something.
        assert!(crate::config::Config::default().plugins_carried);
        let (old, _) = crate::config::Config::parse("{}").unwrap();
        assert!(!old.plugins_carried);
    }
}
