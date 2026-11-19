//! The plugins on disk: each in `<data dir>/ardoise/plugins/<id>/` with
//! its `plugin.json`, and `installs.json` beside them saying where each came
//! from. What a plugin keeps for itself is in `plugin-data/<id>/`, which an
//! uninstall leaves.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use ardoise_plugin::manifest::{self, Manifest};
use serde::{Deserialize, Serialize};

/// The largest archive worth unpacking (the database plugin and its drivers
/// are some tens of megabytes).
const MAX_UNPACKED: u64 = 400 * 1024 * 1024;

/// The ids kept for the official plugins.
pub const OFFICIAL_PREFIX: &str = "ardoise.";

pub fn root() -> Option<PathBuf> {
    crate::fsutil::data_dir().map(|d| d.join(crate::fsutil::APP_DIR).join("plugins"))
}

/// The folder of a plugin's own data, made if missing.
pub fn data_dir_of(id: &str) -> Option<PathBuf> {
    let dir = crate::fsutil::data_dir()?
        .join(crate::fsutil::APP_DIR)
        .join("plugin-data")
        .join(id);
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Where a plugin came from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// `owner/name` of the GitHub repository of its releases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Signed by Ardoise.
    #[serde(default)]
    pub official: bool,
    /// A folder of the user's, linked for development.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub linked: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Installed {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub origin: Origin,
}

impl Installed {
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn name(&self) -> String {
        self.manifest.name.get(&rust_i18n::locale()).to_string()
    }

    pub fn description(&self) -> String {
        self.manifest
            .description
            .get(&rust_i18n::locale())
            .to_string()
    }

    /// The program and its arguments, the program found.
    pub fn program(&self) -> Option<(PathBuf, Vec<String>)> {
        let (first, rest) = self.manifest.command.split_first()?;
        let program = if self.manifest.runs_own_file() {
            let path = self.dir.join(first);
            path.is_file().then_some(path)?
        } else {
            crate::lsp::find_executable(first)?
        };
        Some((program, rest.to_vec()))
    }
}

fn installs_file(root: &Path) -> PathBuf {
    root.join("installs.json")
}

fn read_origins(root: &Path) -> BTreeMap<String, Origin> {
    std::fs::read_to_string(installs_file(root))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_origins(root: &Path, origins: &BTreeMap<String, Origin>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(origins).map_err(|e| e.to_string())?;
    crate::fsutil::write_atomic(&installs_file(root), json.as_bytes(), false)
        .map_err(|e| e.to_string())
}

/// The plugins installed, by id. A folder whose manifest is missing or wrong
/// is left out (and said in `broken`).
pub fn installed() -> (Vec<Installed>, Vec<(String, String)>) {
    let Some(root) = root() else {
        return (Vec::new(), Vec::new());
    };
    installed_in(&root)
}

fn installed_in(root: &Path) -> (Vec<Installed>, Vec<(String, String)>) {
    let origins = read_origins(root);
    let mut found = Vec::new();
    let mut broken = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return (found, broken);
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !manifest::valid_id(&name) {
            continue;
        }
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let read = std::fs::read_to_string(dir.join(manifest::FILE))
            .map_err(|e| e.to_string())
            .and_then(|json| Manifest::parse(&json).map_err(|e| e.to_string()));
        match read {
            Ok(m) if m.id == name => {
                let origin = origins.get(&name).cloned().unwrap_or_default();
                // An official id is only trusted with an official origin.
                if m.id.starts_with(OFFICIAL_PREFIX) && !origin.official && origin.linked.is_none()
                {
                    broken.push((name, t!("plugins.err_official_id").to_string()));
                    continue;
                }
                found.push(Installed {
                    manifest: m,
                    dir,
                    origin,
                });
            }
            Ok(_) => broken.push((name, t!("plugins.err_id_mismatch").to_string())),
            Err(e) => broken.push((name, e)),
        }
    }
    found.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    broken.sort();
    (found, broken)
}

/// The manifest inside an archive (`.tar.gz`), without unpacking it.
pub fn manifest_in_archive(bytes: &[u8]) -> Result<Manifest, String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        if is_manifest_path(&path) {
            let mut json = String::new();
            entry
                .by_ref()
                .take(1024 * 1024)
                .read_to_string(&mut json)
                .map_err(|e| e.to_string())?;
            return Manifest::parse(&json).map_err(|e| e.to_string());
        }
    }
    Err(t!("plugins.err_no_manifest").to_string())
}

/// `plugin.json` at the top, or in the one folder at the top.
fn is_manifest_path(path: &Path) -> bool {
    let parts: Vec<_> = path
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    match parts.as_slice() {
        [Component::Normal(name)] => *name == manifest::FILE,
        [Component::Normal(_), Component::Normal(name)] => *name == manifest::FILE,
        _ => false,
    }
}

/// Unpacks a plugin's archive and puts it in place of the one with its id,
/// if any. `expect` is the id it must have (an official plugin of the list).
pub fn install_archive(
    bytes: &[u8],
    origin: Origin,
    expect: Option<&str>,
) -> Result<Installed, String> {
    let root = root().ok_or_else(|| t!("plugins.err_platform").to_string())?;
    install_archive_in(&root, bytes, origin, expect)
}

fn install_archive_in(
    root: &Path,
    bytes: &[u8],
    origin: Origin,
    expect: Option<&str>,
) -> Result<Installed, String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let staging = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    unpack(bytes, staging.path())?;

    // The plugin is the top of the archive, or its one folder.
    let top = if staging.path().join(manifest::FILE).is_file() {
        staging.path().to_path_buf()
    } else {
        let mut dirs = std::fs::read_dir(staging.path())
            .map_err(|e| e.to_string())?
            .flatten()
            .filter(|e| e.path().is_dir());
        match (dirs.next(), dirs.next()) {
            (Some(only), None) if only.path().join(manifest::FILE).is_file() => only.path(),
            _ => return Err(t!("plugins.err_no_manifest").to_string()),
        }
    };
    let json = std::fs::read_to_string(top.join(manifest::FILE)).map_err(|e| e.to_string())?;
    let manifest = Manifest::parse(&json).map_err(|e| e.to_string())?;
    check_identity(root, &manifest, &origin, expect)?;

    let installed = Installed {
        dir: root.join(&manifest.id),
        manifest,
        origin: origin.clone(),
    };
    if installed.manifest.runs_own_file() {
        let program = top.join(&installed.manifest.command[0]);
        if !program.is_file() {
            return Err(t!("plugins.err_no_program").to_string());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
    }

    // The old version moves aside, the new one takes its name, the old one
    // goes: a failure in between leaves one or the other, never a mix.
    let dest = &installed.dir;
    let aside = root.join(format!(".old-{}", installed.manifest.id));
    let _ = remove_any(&aside);
    let had_old = dest.symlink_metadata().is_ok();
    if had_old {
        std::fs::rename(dest, &aside).map_err(|e| e.to_string())?;
    }
    if let Err(e) = std::fs::rename(&top, dest) {
        if had_old {
            let _ = std::fs::rename(&aside, dest);
        }
        return Err(e.to_string());
    }
    let _ = remove_any(&aside);

    let mut origins = read_origins(root);
    origins.insert(installed.manifest.id.clone(), origin);
    write_origins(root, &origins)?;
    Ok(installed)
}

/// Links a folder of the user's as a plugin, for its author to try it as
/// they build it. The folder stays theirs: an uninstall only removes the link.
#[cfg(unix)]
pub fn link_folder(folder: &Path) -> Result<Installed, String> {
    let root = root().ok_or_else(|| t!("plugins.err_platform").to_string())?;
    link_folder_in(&root, folder)
}

#[cfg(unix)]
fn link_folder_in(root: &Path, folder: &Path) -> Result<Installed, String> {
    let folder = folder.canonicalize().map_err(|e| e.to_string())?;
    let json = std::fs::read_to_string(folder.join(manifest::FILE))
        .map_err(|_| t!("plugins.err_no_manifest").to_string())?;
    let manifest = Manifest::parse(&json).map_err(|e| e.to_string())?;
    let origin = Origin {
        linked: Some(folder.clone()),
        ..Origin::default()
    };
    check_identity(root, &manifest, &origin, None)?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let dest = root.join(&manifest.id);
    remove_any(&dest).map_err(|e| e.to_string())?;
    std::os::unix::fs::symlink(&folder, &dest).map_err(|e| e.to_string())?;
    let mut origins = read_origins(root);
    origins.insert(manifest.id.clone(), origin.clone());
    write_origins(root, &origins)?;
    Ok(Installed {
        manifest,
        dir: dest,
        origin,
    })
}

/// A plugin may take the place of another one only if it comes from the
/// same place, and only the official ones have official ids.
fn check_identity(
    root: &Path,
    manifest: &Manifest,
    origin: &Origin,
    expect: Option<&str>,
) -> Result<(), String> {
    if expect.is_some_and(|id| id != manifest.id) {
        return Err(t!("plugins.err_id_mismatch").to_string());
    }
    if manifest.id.starts_with(OFFICIAL_PREFIX) && !origin.official && origin.linked.is_none() {
        return Err(t!("plugins.err_official_id").to_string());
    }
    if let Some(old) = read_origins(root).get(&manifest.id) {
        let replaced_by_other = old.linked.is_none()
            && origin.linked.is_none()
            && (old.repository != origin.repository || old.official != origin.official);
        if replaced_by_other && root.join(&manifest.id).exists() {
            let from = old.repository.clone().unwrap_or_default();
            return Err(t!("plugins.err_taken", from = from.as_str()).to_string());
        }
    }
    Ok(())
}

/// Removes a plugin (its folder, or the link to a folder of the user's).
pub fn uninstall(id: &str) -> Result<(), String> {
    let root = root().ok_or_else(|| t!("plugins.err_platform").to_string())?;
    uninstall_in(&root, id)
}

fn uninstall_in(root: &Path, id: &str) -> Result<(), String> {
    if !manifest::valid_id(id) {
        return Err(t!("plugins.err_id_mismatch").to_string());
    }
    remove_any(&root.join(id)).map_err(|e| e.to_string())?;
    let mut origins = read_origins(root);
    if origins.remove(id).is_some() {
        write_origins(root, &origins)?;
    }
    Ok(())
}

/// Deletes what a plugin kept (its settings, its history, its connections):
/// it is removed with its data.
pub fn remove_data(id: &str) -> Result<(), String> {
    if !manifest::valid_id(id) {
        return Err(t!("plugins.err_id_mismatch").to_string());
    }
    let Some(base) = crate::fsutil::data_dir() else {
        return Ok(());
    };
    remove_any(
        &base
            .join(crate::fsutil::APP_DIR)
            .join("plugin-data")
            .join(id),
    )
    .map_err(|e| e.to_string())
}

/// Removes a file, a link (not what it points to) or a folder.
fn remove_any(path: &Path) -> std::io::Result<()> {
    match path.symlink_metadata() {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Unpacks a `.tar.gz` into `dest`: only files and folders, inside `dest`,
/// and not more than [`MAX_UNPACKED`] bytes in all.
fn unpack(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut total: u64 = 0;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            // Links could point out of the plugin's folder; the rest is noise.
            if kind.is_symlink() || kind.is_hard_link() {
                return Err(t!("plugins.err_archive").to_string());
            }
            continue;
        }
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let safe = path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
        if !safe {
            return Err(t!("plugins.err_archive").to_string());
        }
        total = total.saturating_add(entry.size());
        if total > MAX_UNPACKED {
            return Err(t!("plugins.err_archive").to_string());
        }
        // `unpack_in` refuses what would land outside `dest`, too.
        if !entry.unpack_in(dest).map_err(|e| e.to_string())? {
            return Err(t!("plugins.err_archive").to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn manifest_json(id: &str, version: &str) -> String {
        format!(
            r#"{{"id": "{id}", "name": "Test", "version": "{version}", "api": 1,
                "command": ["bin/run"],
                "contributes": {{"panels": [{{"id": "main", "title": "Main"}}]}}}}"#
        )
    }

    /// A `.tar.gz` of these files (path, content).
    pub(crate) fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *content).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn plugin(id: &str, version: &str) -> Vec<u8> {
        let json = manifest_json(id, version);
        archive(&[
            ("plugin.json", json.as_bytes()),
            ("bin/run", b"#!/bin/sh\n"),
        ])
    }

    fn from(repo: &str) -> Origin {
        Origin {
            repository: Some(repo.into()),
            ..Origin::default()
        }
    }

    #[test]
    fn a_plugin_installs_lists_updates_and_uninstalls() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        let one = install_archive_in(
            root,
            &plugin("acme.hello", "0.1.0"),
            from("acme/hello"),
            None,
        )
        .unwrap();
        assert_eq!(one.dir, root.join("acme.hello"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(root.join("acme.hello/bin/run"))
                .unwrap()
                .permissions();
            assert_eq!(mode.mode() & 0o111, 0o111);
        }
        install_archive_in(
            root,
            &plugin("acme.hello", "0.2.0"),
            from("acme/hello"),
            None,
        )
        .unwrap();
        let (found, broken) = installed_in(root);
        assert!(broken.is_empty());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].manifest.version, "0.2.0");
        assert_eq!(found[0].origin, from("acme/hello"));
        // Nothing of the staging or of the old version is left.
        let names: Vec<_> = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 2, "{names:?}");

        uninstall_in(root, "acme.hello").unwrap();
        assert!(installed_in(root).0.is_empty());
        assert!(read_origins(root).is_empty());
    }

    #[test]
    fn a_plugin_in_a_folder_of_the_archive_is_found() {
        let root = tempfile::tempdir().unwrap();
        let json = manifest_json("acme.hello", "1.0.0");
        let bytes = archive(&[
            ("hello/plugin.json", json.as_bytes()),
            ("hello/bin/run", b"x"),
        ]);
        assert_eq!(manifest_in_archive(&bytes).unwrap().id, "acme.hello");
        install_archive_in(root.path(), &bytes, from("acme/hello"), None).unwrap();
        assert!(root.path().join("acme.hello/bin/run").is_file());
    }

    /// Made as the release workflow of the template makes it:
    /// `tar czf <archive> -C dist/plugin .` (entries start with `./`).
    #[cfg(unix)]
    #[test]
    fn an_archive_made_by_the_release_workflow_installs() {
        let work = tempfile::tempdir().unwrap();
        let dist = work.path().join("dist/plugin");
        std::fs::create_dir_all(dist.join("bin")).unwrap();
        std::fs::write(dist.join("plugin.json"), manifest_json("acme.tar", "1.0.0")).unwrap();
        std::fs::write(dist.join("bin/run"), "#!/bin/sh\n").unwrap();
        let archive = work.path().join("p.tar.gz");
        let made = std::process::Command::new("tar")
            .arg("czf")
            .arg(&archive)
            .arg("-C")
            .arg(&dist)
            .arg(".")
            .status();
        if !made.is_ok_and(|s| s.success()) {
            return;
        }
        let bytes = std::fs::read(&archive).unwrap();
        assert_eq!(manifest_in_archive(&bytes).unwrap().id, "acme.tar");
        let root = tempfile::tempdir().unwrap();
        let installed = install_archive_in(root.path(), &bytes, from("acme/tar"), None).unwrap();
        assert!(installed.dir.join("bin/run").is_file());
        assert!(installed.program().is_some());
    }

    #[test]
    fn archives_that_reach_out_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let json = manifest_json("acme.hello", "1.0.0");
        // `append_data` refuses `..`: the header is written by hand.
        let mut header = tar::Header::new_gnu();
        header.set_size(1);
        header.set_mode(0o644);
        header.as_gnu_mut().unwrap().name[..10].copy_from_slice(b"../evil.sh");
        header.set_cksum();
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let mut first = tar::Header::new_gnu();
        first.set_size(json.len() as u64);
        first.set_mode(0o644);
        builder
            .append_data(&mut first, "plugin.json", json.as_bytes())
            .unwrap();
        builder.append(&header, &b"x"[..]).unwrap();
        let bytes = builder.into_inner().unwrap().finish().unwrap();
        assert!(install_archive_in(root.path(), &bytes, from("a/b"), None).is_err());
        assert!(!root.path().join("evil.sh").exists());
        assert!(!root.path().parent().unwrap().join("evil.sh").exists());

        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let mut link = tar::Header::new_gnu();
        link.set_entry_type(tar::EntryType::Symlink);
        link.set_size(0);
        builder
            .append_link(&mut link, "bin/run", "/bin/sh")
            .unwrap();
        let bytes = builder.into_inner().unwrap().finish().unwrap();
        assert!(install_archive_in(root.path(), &bytes, from("a/b"), None).is_err());
    }

    #[test]
    fn identities_are_kept() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path();
        // Only the official plugins have official ids.
        assert!(
            install_archive_in(root, &plugin("ardoise.git", "1.0.0"), from("x/y"), None).is_err()
        );
        let official = Origin {
            repository: Some("ardoise-editor/plugin-git".into()),
            official: true,
            linked: None,
        };
        install_archive_in(
            root,
            &plugin("ardoise.git", "1.0.0"),
            official.clone(),
            None,
        )
        .unwrap();
        // Another repository cannot take its place.
        assert!(
            install_archive_in(root, &plugin("ardoise.git", "1.0.1"), from("x/y"), None).is_err()
        );
        // The id must be the one the list says.
        assert!(install_archive_in(
            root,
            &plugin("ardoise.lint", "1.0.0"),
            official,
            Some("ardoise.git")
        )
        .is_err());
        // Without the program it starts, it is no plugin.
        let json = manifest_json("acme.empty", "1.0.0");
        let bytes = archive(&[("plugin.json", json.as_bytes())]);
        assert!(install_archive_in(root, &bytes, from("a/b"), None).is_err());
        assert_eq!(installed_in(root).0.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_folder_stays_when_it_is_uninstalled() {
        let root = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        std::fs::write(
            work.path().join("plugin.json"),
            manifest_json("acme.dev", "0.0.1"),
        )
        .unwrap();
        let linked = link_folder_in(root.path(), work.path()).unwrap();
        assert!(linked.origin.linked.is_some());
        assert_eq!(installed_in(root.path()).0.len(), 1);
        uninstall_in(root.path(), "acme.dev").unwrap();
        assert!(work.path().join("plugin.json").is_file());
        assert!(installed_in(root.path()).0.is_empty());
    }
}
