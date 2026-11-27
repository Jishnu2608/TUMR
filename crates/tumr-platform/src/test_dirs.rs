//! TEST-004: one place for the folders unit tests create, so they do not pile up in `%TEMP%`.
//!
//! Every folder is made under `%TEMP%\tumr-tests\`. The first call in a test process removes the
//! folders there that are more than two hours old: whatever earlier runs left behind (a test that
//! failed half-way, or one that never cleans up) disappears with the next run, and folders of a
//! run still going are never touched.
//!
//! For tests only; the product never writes here.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;
use std::time::{Duration, SystemTime};

/// Folders older than this are leftovers of an earlier run.
const STALE: Duration = Duration::from_secs(2 * 60 * 60);

/// `%TEMP%\tumr-tests`.
#[must_use]
pub fn root() -> PathBuf {
    std::env::temp_dir().join("tumr-tests")
}

/// Removes the entries of `root` last changed before `now - STALE`. Returns how many went.
#[must_use = "how many leftovers were removed"]
pub fn sweep(root: &Path, now: SystemTime) -> usize {
    let mut removed = 0;
    for e in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > STALE));
        if old {
            let p = e.path();
            let gone = if p.is_dir() {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p)
            };
            if gone.is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

/// A new, empty folder `%TEMP%\tumr-tests\<tag>-<pid>-<n>` (unique within the test process).
#[must_use]
pub fn fresh(tag: &str) -> PathBuf {
    static SWEEP: Once = Once::new();
    static N: AtomicUsize = AtomicUsize::new(0);
    let root = root();
    SWEEP.call_once(|| {
        let _ = sweep(&root, SystemTime::now());
    });
    let d = root.join(format!(
        "{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::create_dir_all(&d);
    d
}

#[cfg(test)]
mod tests {
    use super::{fresh, sweep, STALE};
    use std::time::{Duration, SystemTime};

    #[test]
    fn fresh_folders_are_unique_and_empty() {
        let a = fresh("td");
        let b = fresh("td");
        assert_ne!(a, b);
        assert!(a.is_dir() && std::fs::read_dir(&a).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&a);
        let _ = std::fs::remove_dir_all(&b);
    }

    #[test]
    fn sweep_removes_only_old_entries() {
        let root = std::env::temp_dir().join(format!("tumr-tests-sweep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::write(root.join("b.txt"), b"x").unwrap();
        // "now" just inside the limit: nothing is old yet
        assert_eq!(
            sweep(&root, SystemTime::now() + STALE - Duration::from_secs(60)),
            0
        );
        // three hours later both are leftovers
        assert_eq!(
            sweep(&root, SystemTime::now() + STALE + Duration::from_secs(3600)),
            2
        );
        assert!(std::fs::read_dir(&root).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
