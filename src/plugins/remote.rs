//! Where plugins come from: the list of the official ones, and the GitHub
//! releases of each plugin.
//!
//! A release of a plugin holds one archive per system,
//! `<anything>-<target>.tar.gz` (`x86_64-linux`, `aarch64-apple-darwin`), or
//! one for all, `<anything>-any.tar.gz` (a plugin written in a scripting
//! language), and `SHA256SUMS` listing them. The official plugins also sign
//! it (`SHA256SUMS.sig`) with the key the app's own releases are signed
//! with.

use std::time::Duration;

use ardoise_plugin::manifest::{parse_version, Localized};
use serde::{Deserialize, Serialize};

/// The list of the official plugins.
pub const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/ardoise-editor/plugins/main/registry.json";

/// The organization of the official plugins.
pub const OFFICIAL_OWNER: &str = "ardoise-editor";

/// The guide to writing a plugin.
pub const GUIDE_URL: &str = "https://ardoise-editor.github.io/ardoise/plugins.html";

/// The largest archive downloaded.
const MAX_ARCHIVE: usize = 200 * 1024 * 1024;

/// The system the archives are for, as their names say it.
pub fn target() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-linux")
    } else {
        None
    }
}

/// A plugin of the official list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listed {
    pub id: String,
    pub name: Localized,
    #[serde(default)]
    pub description: Localized,
    /// `owner/name`.
    pub repository: String,
}

#[derive(Debug, Deserialize)]
struct Registry {
    plugins: Vec<Listed>,
}

/// The list of the official plugins: only those of the official
/// organization, with official ids.
pub fn parse_registry(json: &str) -> Result<Vec<Listed>, String> {
    let registry: Registry = serde_json::from_str(json).map_err(|e| e.to_string())?;
    Ok(registry
        .plugins
        .into_iter()
        .filter(|p| {
            p.id.starts_with(super::store::OFFICIAL_PREFIX)
                && ardoise_plugin::manifest::valid_id(&p.id)
                && parse_repo(&p.repository)
                    .is_some_and(|r| r.split('/').next() == Some(OFFICIAL_OWNER))
        })
        .collect())
}

pub async fn fetch_registry() -> Result<Vec<Listed>, String> {
    let client = crate::updater::client(Duration::from_secs(20))?;
    let bytes = crate::updater::fetch_limited(&client, REGISTRY_URL, 1024 * 1024).await?;
    parse_registry(&String::from_utf8_lossy(&bytes))
}

/// `owner/name` from what the user typed: `owner/name`, or the address of
/// the repository (`https://github.com/owner/name`, with `.git` or more
/// after it).
pub fn parse_repo(input: &str) -> Option<String> {
    let s = input.trim().trim_end_matches('/');
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s);
    let s = s
        .strip_prefix("www.")
        .unwrap_or(s)
        .strip_prefix("github.com/")
        .unwrap_or(s);
    let mut parts = s.split('/');
    let owner = parts.next()?;
    let name = parts.next()?.trim_end_matches(".git");
    let good = |p: &str| {
        !p.is_empty()
            && p.len() <= 100
            && p != "."
            && p != ".."
            && p.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    (good(owner) && good(name)).then(|| format!("{owner}/{name}"))
}

/// The newest release of a plugin, and its files for this system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub repository: String,
    /// `1.2.0`, from the tag (`v1.2.0`).
    pub version: String,
    pub archive: Asset,
    pub sums: Asset,
    pub signature: Option<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Debug, Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

/// What GitHub said of the latest release of `repository`: its archive for
/// `target`, or the one for any system.
pub fn parse_release(json: &str, repository: &str, target: &str) -> Result<Release, String> {
    let release: ApiRelease = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_string();
    if parse_version(&version).is_none() {
        return Err(t!("plugins.err_release").to_string());
    }
    // Only files of this repository's releases.
    let base = format!("https://github.com/{repository}/releases/download/");
    let assets: Vec<Asset> = release
        .assets
        .into_iter()
        .filter(|a| a.browser_download_url.starts_with(&base))
        .map(|a| Asset {
            name: a.name,
            url: a.browser_download_url,
        })
        .collect();
    let find = |pred: &dyn Fn(&str) -> bool| assets.iter().find(|a| pred(&a.name)).cloned();
    let archive = find(&|n| n.ends_with(&format!("-{target}.tar.gz")))
        .or_else(|| find(&|n| n.ends_with("-any.tar.gz")))
        .ok_or_else(|| t!("plugins.err_no_archive").to_string())?;
    let sums = find(&|n| n == "SHA256SUMS").ok_or_else(|| t!("plugins.err_no_sums").to_string())?;
    let signature = find(&|n| n == "SHA256SUMS.sig");
    Ok(Release {
        repository: repository.to_string(),
        version,
        archive,
        sums,
        signature,
    })
}

pub async fn latest_release(repository: String) -> Result<Release, String> {
    let target = target().ok_or_else(|| t!("plugins.err_platform").to_string())?;
    let client = crate::updater::client(Duration::from_secs(20))?;
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(t!("plugins.err_no_release", repo = repository.as_str()).to_string());
    }
    let response = response.error_for_status().map_err(|e| e.to_string())?;
    let json = response.text().await.map_err(|e| e.to_string())?;
    parse_release(&json, &repository, target)
}

/// The archive of a release, checked against its `SHA256SUMS` and, for an
/// official plugin, the signature of those.
pub async fn download(release: Release, official: bool) -> Result<Vec<u8>, String> {
    use crate::updater::{expected_hash, fetch_limited, sha256_hex, verify_signature, MAX_SUMS};
    let client = crate::updater::client(Duration::from_secs(300))?;
    let sums = fetch_limited(&client, &release.sums.url, MAX_SUMS).await?;
    if official {
        let signature = release
            .signature
            .as_ref()
            .ok_or_else(|| t!("plugins.err_signature").to_string())?;
        let sig = fetch_limited(&client, &signature.url, MAX_SUMS).await?;
        if !verify_signature(&sums, &sig) {
            return Err(t!("plugins.err_signature").to_string());
        }
    }
    let expected = expected_hash(&String::from_utf8_lossy(&sums), &release.archive.name)
        .ok_or_else(|| t!("plugins.err_checksum").to_string())?;
    let bytes = fetch_limited(&client, &release.archive.url, MAX_ARCHIVE).await?;
    if sha256_hex(&bytes) != expected {
        return Err(t!("plugins.err_checksum").to_string());
    }
    Ok(bytes)
}

/// Whether `latest` is newer than `current` (both `MAJOR.MINOR.PATCH`).
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real registry and releases: every official plugin downloads for
    /// this machine, its signature checked. Over the network, so asked for:
    /// `cargo test -- --ignored every_official_plugin`.
    #[test]
    #[ignore]
    fn every_official_plugin_downloads_with_a_valid_signature() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let listed = fetch_registry().await.expect("the registry");
            assert!(listed.len() >= 9, "{} plugins listed", listed.len());
            for plugin in listed {
                let release = latest_release(plugin.repository.clone())
                    .await
                    .unwrap_or_else(|e| panic!("{}: {e}", plugin.repository));
                let bytes = download(release, true)
                    .await
                    .unwrap_or_else(|e| panic!("{}: {e}", plugin.repository));
                assert!(!bytes.is_empty(), "{}", plugin.repository);
            }
        });
    }

    #[test]
    fn repositories_are_read_from_what_people_paste() {
        for input in [
            "acme/hello",
            "https://github.com/acme/hello",
            "https://github.com/acme/hello/",
            "github.com/acme/hello.git",
            "https://www.github.com/acme/hello/releases",
        ] {
            assert_eq!(parse_repo(input).as_deref(), Some("acme/hello"), "{input}");
        }
        for input in ["", "acme", "acme/", "../x", "a b/c", "acme/../x"] {
            assert_eq!(parse_repo(input), None, "{input}");
        }
    }

    #[test]
    fn the_registry_keeps_only_official_plugins() {
        let json = r#"{"version": 1, "plugins": [
            {"id": "ardoise.git", "name": {"en": "Git"}, "repository": "ardoise-editor/plugin-git"},
            {"id": "ardoise.evil", "name": "Evil", "repository": "evil/plugin"},
            {"id": "acme.hello", "name": "Hello", "repository": "ardoise-editor/hello"}
        ]}"#;
        let listed = parse_registry(json).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name.get("fr"), "Git");
    }

    #[test]
    fn a_release_gives_the_archive_of_this_system() {
        let json = r#"{"tag_name": "v1.2.0", "assets": [
            {"name": "hello-1.2.0-x86_64-linux.tar.gz", "browser_download_url": "https://github.com/acme/hello/releases/download/v1.2.0/hello-1.2.0-x86_64-linux.tar.gz"},
            {"name": "hello-1.2.0-any.tar.gz", "browser_download_url": "https://github.com/acme/hello/releases/download/v1.2.0/hello-1.2.0-any.tar.gz"},
            {"name": "SHA256SUMS", "browser_download_url": "https://github.com/acme/hello/releases/download/v1.2.0/SHA256SUMS"},
            {"name": "SHA256SUMS.sig", "browser_download_url": "https://evil.example/SHA256SUMS.sig"}
        ]}"#;
        let linux = parse_release(json, "acme/hello", "x86_64-linux").unwrap();
        assert_eq!(linux.version, "1.2.0");
        assert_eq!(linux.archive.name, "hello-1.2.0-x86_64-linux.tar.gz");
        // A file from elsewhere is not one of the release's.
        assert_eq!(linux.signature, None);
        let mac = parse_release(json, "acme/hello", "aarch64-apple-darwin").unwrap();
        assert_eq!(mac.archive.name, "hello-1.2.0-any.tar.gz");
        let no_sums = json.replace("\"SHA256SUMS\"", "\"other\"");
        assert!(parse_release(&no_sums, "acme/hello", "x86_64-linux").is_err());
        let bad_tag = json.replace("v1.2.0\"", "latest\"");
        assert!(parse_release(&bad_tag, "acme/hello", "x86_64-linux").is_err());
    }

    #[test]
    fn newer_is_by_number() {
        assert!(is_newer("1.10.0", "1.9.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("garbage", "1.0.0"));
    }
}
