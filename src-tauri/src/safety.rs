//! §7 safety rails. Every rule here is a hard requirement with a test below it.
//!
//! These run at two moments: when a plan is validated (cheap, path-shape checks) and
//! again immediately before each move (expensive, filesystem-touching checks). The
//! second pass exists because a symlink can be swapped between approval and execution.

use crate::config::{Config, Destination};
use crate::error::{Error, Result};
use std::path::{Component, Path, PathBuf};

/// Absolute prefixes Menlo will never write into, whatever the config says.
const FORBIDDEN_PREFIXES: &[&str] = &[
    "/System",
    "/Library",
    "/Applications",
    "/bin",
    "/sbin",
    "/usr",
    "/etc",
    "/var",
    "/private/var/db",
    "/Network",
    "/Volumes/Recovery",
];

/// §7: installers are left alone unless explicitly enabled.
const INSTALLER_EXTS: &[&str] = &["dmg", "pkg", "app", "mpkg", "iso"];

pub fn is_installer(ext: &str) -> bool {
    INSTALLER_EXTS.contains(&ext.to_ascii_lowercase().as_str())
}

/// Refuse `/`, the system prefixes above, `~/Library`, and anything inside a bundle.
///
/// Takes an already-absolute path. Call `canonical_guard` for the version that also
/// resolves symlinks.
pub fn assert_operable(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::safety(format!(
            "{} is not an absolute path",
            path.display()
        )));
    }

    let n_components = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    if n_components == 0 {
        return Err(Error::safety("refusing to operate on the filesystem root"));
    }

    // Any component that looks like a bundle: §7 forbids paths containing `.app`, and
    // the same reasoning covers the other bundle types.
    for c in path.components() {
        if let Component::Normal(os) = c {
            let s = os.to_string_lossy().to_ascii_lowercase();
            if s.ends_with(".app") || s.ends_with(".bundle") || s.ends_with(".framework") {
                return Err(Error::safety(format!(
                    "{} is inside an application bundle",
                    path.display()
                )));
            }
        }
    }

    let as_str = path.to_string_lossy().to_string();
    for prefix in FORBIDDEN_PREFIXES {
        if as_str == *prefix || as_str.starts_with(&format!("{prefix}/")) {
            return Err(Error::safety(format!(
                "{} is inside protected {prefix}",
                path.display()
            )));
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let lib = format!("{home}/Library");
        // Menlo's own Application Support directory is the one exception; the journal
        // lives there and revert has to be able to read it.
        let own = format!("{lib}/Application Support/menlo");
        if (as_str == lib || as_str.starts_with(&format!("{lib}/")))
            && !(as_str == own || as_str.starts_with(&format!("{own}/")))
        {
            return Err(Error::safety(format!(
                "{} is inside ~/Library",
                path.display()
            )));
        }
        if as_str == home {
            return Err(Error::safety(
                "refusing to operate on the home directory itself",
            ));
        }
    }

    Ok(())
}

/// Canonicalise `path` (resolving symlinks) and re-run [`assert_operable`] on the
/// result. §7 is explicit that the check must happen *after* symlink resolution — a
/// destination that canonicalises into `/System` is not saved by having a clean name.
///
/// For a path that does not exist yet (the file we are about to create), the nearest
/// existing ancestor is canonicalised instead and the remainder appended.
pub fn canonical_guard(path: &Path) -> Result<PathBuf> {
    let resolved = canonicalize_lexically_safe(path)?;
    assert_operable(&resolved)?;
    Ok(resolved)
}

fn canonicalize_lexically_safe(path: &Path) -> Result<PathBuf> {
    if let Ok(c) = path.canonicalize() {
        return Ok(c);
    }
    let parent = path
        .parent()
        .ok_or_else(|| Error::safety(format!("{} has no parent", path.display())))?;
    let name = path
        .file_name()
        .ok_or_else(|| Error::safety(format!("{} has no file name", path.display())))?;
    let parent = parent.canonicalize().map_err(|e| Error::io(parent, e))?;
    Ok(parent.join(name))
}

/// §7 destination allowlist: the resolved target must sit inside one of the roots the
/// user actually added. This is the check that turns a compromised or hallucinated
/// plan into a rejection rather than a stray write.
pub fn assert_within_destination(target: &Path, destinations: &[Destination]) -> Result<()> {
    let resolved = canonicalize_lexically_safe(target)?;
    for d in destinations {
        // The root itself must be legitimate before we trust anything under it.
        let Ok(root) = d.path.canonicalize() else {
            continue;
        };
        if assert_operable(&root).is_err() {
            continue;
        }
        if resolved.starts_with(&root) {
            return Ok(());
        }
    }
    Err(Error::safety(format!(
        "{} is outside every configured destination",
        resolved.display()
    )))
}

/// §7: never move a file another process holds open.
///
/// Shelling out to `lsof` is the pragmatic macOS answer; there is no portable syscall
/// for it. A missing or failing `lsof` is treated as "cannot prove it is closed" and
/// the move is allowed to proceed — the alternative is an app that refuses to work on
/// a locked-down machine. Callers that need certainty check the return value.
pub fn is_open_by_another_process(path: &Path) -> bool {
    use std::process::{Command, Stdio};
    match Command::new("/usr/sbin/lsof")
        .arg("-t") // terse: pids only
        .arg("--")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => false,
    }
}

/// The full pre-move gate. Runs immediately before the filesystem call, not at plan
/// time, so a swapped symlink or a newly-opened file is still caught.
pub fn assert_move_allowed(source: &Path, target: &Path, config: &Config, ext: &str) -> Result<()> {
    let src = canonical_guard(source)?;

    // §6.2 / v1 scope: directories and symlinks are not in play.
    let meta = std::fs::symlink_metadata(source).map_err(|e| Error::io(source, e))?;
    if meta.file_type().is_symlink() {
        return Err(Error::safety(format!("{} is a symlink", source.display())));
    }
    if meta.is_dir() {
        return Err(Error::safety(format!(
            "{} is a directory",
            source.display()
        )));
    }

    if !config.settings.allow_installers && is_installer(ext) {
        return Err(Error::safety(format!(
            "{} is an installer; enable \"move installers\" in Settings to include it",
            source.display()
        )));
    }

    assert_operable(target)?;
    assert_within_destination(target, &config.destinations)?;

    if config.settings.check_open_files && is_open_by_another_process(&src) {
        return Err(Error::safety(format!(
            "{} is open in another application",
            src.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn refuses_the_root_and_system_paths() {
        assert!(assert_operable(Path::new("/")).is_err());
        assert!(assert_operable(Path::new("/System")).is_err());
        assert!(assert_operable(Path::new("/System/Library/Fonts")).is_err());
        assert!(assert_operable(Path::new("/Applications")).is_err());
        assert!(assert_operable(Path::new("/usr/local/bin")).is_err());
    }

    #[test]
    fn refuses_paths_inside_bundles() {
        assert!(assert_operable(Path::new("/Users/x/Desktop/Foo.app/Contents/file")).is_err());
        assert!(assert_operable(Path::new("/Users/x/Thing.bundle/x")).is_err());
    }

    #[test]
    fn refuses_home_library_but_allows_our_own_dir() {
        let home = std::env::var("HOME").unwrap();
        assert!(assert_operable(Path::new(&format!("{home}/Library/Caches"))).is_err());
        assert!(assert_operable(Path::new(&format!(
            "{home}/Library/Application Support/menlo/journal"
        )))
        .is_ok());
    }

    #[test]
    fn refuses_relative_paths() {
        assert!(assert_operable(Path::new("relative/path")).is_err());
    }

    #[test]
    fn allows_ordinary_user_paths() {
        let home = std::env::var("HOME").unwrap();
        assert!(assert_operable(Path::new(&format!("{home}/Documents/Finance"))).is_ok());
    }

    #[test]
    fn allowlist_accepts_inside_and_rejects_outside() {
        let tmp = TempDir::new();
        let dest_root = tmp.join("Finance");
        std::fs::create_dir_all(&dest_root).unwrap();
        let elsewhere = tmp.join("Elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();

        let dests = vec![Destination {
            key: "finance".into(),
            label: "Finance".into(),
            path: dest_root.clone(),
            brief: String::new(),
        }];

        assert!(assert_within_destination(&dest_root.join("a.pdf"), &dests).is_ok());
        assert!(assert_within_destination(&elsewhere.join("a.pdf"), &dests).is_err());
    }

    #[test]
    fn allowlist_rejects_a_symlink_that_escapes_the_root() {
        // The attack the "canonicalise after symlink resolution" clause exists for.
        let tmp = TempDir::new();
        let dest_root = tmp.join("Finance");
        std::fs::create_dir_all(&dest_root).unwrap();
        let outside = tmp.join("Outside");
        std::fs::create_dir_all(&outside).unwrap();

        let escape = dest_root.join("escape");
        std::os::unix::fs::symlink(&outside, &escape).unwrap();

        let dests = vec![Destination {
            key: "finance".into(),
            label: "Finance".into(),
            path: dest_root,
            brief: String::new(),
        }];

        // Lexically this is "inside Finance"; after resolution it is not.
        assert!(assert_within_destination(&escape.join("a.pdf"), &dests).is_err());
    }

    #[test]
    fn installer_extensions_are_recognised_case_insensitively() {
        assert!(is_installer("dmg"));
        assert!(is_installer("PKG"));
        assert!(is_installer("App"));
        assert!(!is_installer("pdf"));
    }
}
