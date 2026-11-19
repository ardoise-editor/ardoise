//! Crash-safe file writes.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Where the app keeps its data (session, bookmarks). Tests get a throwaway
/// folder so they never touch the user's files.
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(test)]
    {
        Some(test_dir().join("data"))
    }
    #[cfg(not(test))]
    {
        dirs::data_dir()
    }
}

/// Where the app keeps its settings.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(test)]
    {
        Some(test_dir().join("config"))
    }
    #[cfg(not(test))]
    {
        dirs::config_dir()
    }
}

/// The folder of the app in the data and the config directories.
pub const APP_DIR: &str = "ardoise";

/// What that folder was called before the app was renamed (it was Ardoise).
pub const LEGACY_APP_DIR: &str = "simpleedit";

/// Moves what the app kept under its old name to the new one, once: settings,
/// session, plugins and their data, language servers. Nothing is touched when
/// the new folder exists already, nor when there is no old one. Run before
/// anything reads or writes there.
pub fn migrate_legacy_dirs() {
    let mut bases = vec![data_dir(), config_dir()];
    bases.dedup();
    for base in bases.into_iter().flatten() {
        let _ = move_legacy_dir(&base);
        migrate_legacy_plugins(&base.join(APP_DIR));
    }
}

/// The official plugins were called `simpleedit.<name>`, they are
/// `ardoise.<name>`: what a plugin kept goes to its new name, and the old
/// programs go (they are installed again from Preferences › Plugins).
fn migrate_legacy_plugins(app: &Path) {
    const OLD: &str = "simpleedit.";
    let renamed = |name: &str| name.strip_prefix(OLD).map(|rest| format!("ardoise.{rest}"));
    if let Ok(entries) = std::fs::read_dir(app.join("plugin-data")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(new) = renamed(&name) {
                let target = entry.path().with_file_name(new);
                if !target.exists() {
                    let _ = std::fs::rename(entry.path(), target);
                }
            }
        }
    }
    let plugins = app.join("plugins");
    if let Ok(entries) = std::fs::read_dir(&plugins) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(OLD) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
    let installs = plugins.join("installs.json");
    if let Some(mut list) = std::fs::read_to_string(&installs)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&t).ok())
    {
        let before = list.len();
        list.retain(|id, _| !id.starts_with(OLD));
        if list.len() != before {
            if let Ok(json) = serde_json::to_string_pretty(&list) {
                let _ = write_atomic(&installs, json.as_bytes(), false);
            }
        }
    }
}

/// `<base>/simpleedit` becomes `<base>/ardoise`; whether it moved.
fn move_legacy_dir(base: &Path) -> std::io::Result<bool> {
    let (old, new) = (base.join(LEGACY_APP_DIR), base.join(APP_DIR));
    if !old.is_dir() || new.exists() {
        return Ok(false);
    }
    std::fs::rename(&old, &new)?;
    Ok(true)
}

#[cfg(test)]
fn test_dir() -> PathBuf {
    std::env::temp_dir().join(format!("ardoise-test-{}", std::process::id()))
}

/// Writes `data` to `path` through a temporary sibling file and a `rename`,
/// so a crash mid-write never leaves a truncated target. Symlinks are
/// followed (the link target is replaced, not the link) and an existing
/// file keeps its permissions. With `private`, the file is created `0600`.
///
/// The temporary file is created owner-only and takes the target's
/// permissions before any data is written. A target that the rename would
/// change for others (a hard link, a file of another owner) is rewritten in
/// place instead, and a read-only file is refused. If the directory isn't
/// writable, falls back to a direct write.
pub fn write_atomic(path: &Path, data: &[u8], private: bool) -> std::io::Result<()> {
    let target = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let existing = std::fs::metadata(&target).ok();
    if !private
        && existing
            .as_ref()
            .is_some_and(|m| m.permissions().readonly())
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            t!("status.err_permission").to_string(),
        ));
    }
    #[cfg(unix)]
    if !private && existing.as_ref().is_some_and(is_shared_inode) {
        return write_in_place(&target, data);
    }

    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(".{name}.{}.{nonce}.tmp", std::process::id()));

    let result = (|| {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // A brand-new file gets the usual permissions (minus umask); a
            // replaced one starts owner-only until it takes the old file's.
            opts.mode(if private || existing.is_some() {
                0o600
            } else {
                0o666
            });
        }
        let mut file = opts.open(&tmp)?;
        if !private {
            if let Some(meta) = &existing {
                #[cfg(unix)]
                if !same_owner(&file.metadata()?, meta) {
                    drop(file);
                    let _ = std::fs::remove_file(&tmp);
                    return write_in_place(&target, data);
                }
                file.set_permissions(meta.permissions())?;
            }
        }
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, &target)?;
        sync_dir(dir);
        Ok(())
    })();

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            if !private && e.kind() == std::io::ErrorKind::PermissionDenied {
                std::fs::write(&target, data)
            } else {
                Err(e)
            }
        }
    }
}

/// Truncates and rewrites the file itself (keeps its inode, owner and links).
fn write_in_place(target: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(target)?;
    file.write_all(data)?;
    file.sync_all()
}

#[cfg(unix)]
fn is_shared_inode(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.nlink() > 1
}

#[cfg(unix)]
fn same_owner(ours: &std::fs::Metadata, theirs: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    ours.uid() == theirs.uid() && ours.gid() == theirs.gid()
}

/// Makes the rename itself durable (best effort; not possible everywhere).
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(handle) = std::fs::File::open(dir) {
        let _ = handle.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

/// Moves an unreadable data file aside (`name.bak-<unix time>`) so the next
/// save doesn't overwrite what may still be recoverable. Returns the new path.
pub fn set_aside(path: &Path) -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path.file_name()?.to_string_lossy().into_owned();
    let aside = path.with_file_name(format!("{name}.bak-{stamp}"));
    std::fs::rename(path, &aside).ok().map(|()| aside)
}

/// Removes a file or a folder by sending it to the system trash; when there
/// is no trash (a headless box, another volume), it is deleted for good.
pub fn trash_or_remove(path: &Path) -> std::io::Result<()> {
    // Unit tests delete for good: they must not fill the developer's trash.
    if !cfg!(test) && trash::delete(path).is_ok() {
        return Ok(());
    }
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_content_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, b"new", false).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn private_files_are_owner_only_and_symlinks_are_followed() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        write_atomic(&path, b"{}", true).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);

        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        write_atomic(&link, b"[]", true).unwrap();
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[]");
    }

    #[cfg(unix)]
    #[test]
    fn read_only_files_are_refused_and_left_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ro.txt");
        std::fs::write(&path, "keep").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
        let err = write_atomic(&path, b"new", false).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep");
    }

    #[cfg(unix)]
    #[test]
    fn hard_links_see_the_new_content() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "old").unwrap();
        std::fs::hard_link(&a, &b).unwrap();
        write_atomic(&a, b"new", false).unwrap();
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn a_private_target_keeps_its_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        std::fs::write(&path, "SECRET=1").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        write_atomic(&path, b"SECRET=2", false).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "SECRET=2");
    }

    #[test]
    fn set_aside_keeps_the_unreadable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(&path, "{ broken").unwrap();
        let aside = set_aside(&path).unwrap();
        assert!(!path.exists());
        assert_eq!(std::fs::read_to_string(aside).unwrap(), "{ broken");
    }

    #[test]
    fn what_the_app_kept_under_its_old_name_moves_once() {
        let base = tempfile::tempdir().unwrap();
        let old = base.path().join(LEGACY_APP_DIR);
        std::fs::create_dir_all(old.join("plugins/ardoise.git")).unwrap();
        std::fs::write(old.join("session.json"), "{}").unwrap();
        // Moved, with everything in it.
        assert!(move_legacy_dir(base.path()).unwrap());
        let new = base.path().join(APP_DIR);
        assert!(!old.exists());
        assert!(new.join("session.json").is_file());
        assert!(new.join("plugins/ardoise.git").is_dir());
        // Nothing left to move.
        assert!(!move_legacy_dir(base.path()).unwrap());
        // A new folder that exists wins: the old one is left alone.
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("session.json"), "old").unwrap();
        std::fs::write(new.join("session.json"), "new").unwrap();
        assert!(!move_legacy_dir(base.path()).unwrap());
        assert_eq!(
            std::fs::read_to_string(new.join("session.json")).unwrap(),
            "new"
        );
        assert_eq!(
            std::fs::read_to_string(old.join("session.json")).unwrap(),
            "old"
        );
        // A file with the old name is not a folder to move.
        let other = tempfile::tempdir().unwrap();
        std::fs::write(other.path().join(LEGACY_APP_DIR), "x").unwrap();
        assert!(!move_legacy_dir(other.path()).unwrap());
    }

    #[test]
    fn the_plugins_of_the_old_name_give_their_data_to_the_new_one_and_go() {
        let app = tempfile::tempdir().unwrap();
        let plugins = app.path().join("plugins");
        std::fs::create_dir_all(plugins.join("simpleedit.git")).unwrap();
        std::fs::create_dir_all(plugins.join("acme.hello")).unwrap();
        std::fs::write(
            plugins.join("installs.json"),
            r#"{"simpleedit.git": {"official": true}, "acme.hello": {"official": false}}"#,
        )
        .unwrap();
        let data = app.path().join("plugin-data");
        std::fs::create_dir_all(data.join("simpleedit.git")).unwrap();
        std::fs::write(data.join("simpleedit.git/settings.json"), "mine").unwrap();
        std::fs::create_dir_all(data.join("simpleedit.api")).unwrap();
        std::fs::create_dir_all(data.join("ardoise.api")).unwrap();
        std::fs::write(data.join("ardoise.api/kept"), "already there").unwrap();
        migrate_legacy_plugins(app.path());
        assert_eq!(
            std::fs::read_to_string(data.join("ardoise.git/settings.json")).unwrap(),
            "mine"
        );
        assert!(!data.join("simpleedit.git").exists());
        // A new folder that is there already is left as it is.
        assert!(data.join("ardoise.api/kept").exists() && data.join("simpleedit.api").exists());
        assert!(!plugins.join("simpleedit.git").exists() && plugins.join("acme.hello").exists());
        let installs = std::fs::read_to_string(plugins.join("installs.json")).unwrap();
        assert!(installs.contains("acme.hello") && !installs.contains("simpleedit.git"));
        // Nothing to do the second time.
        migrate_legacy_plugins(app.path());
        migrate_legacy_plugins(&app.path().join("nowhere"));
    }
}
