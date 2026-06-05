use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

/// A running vault watcher. Drop to stop.
pub struct VaultWatcher {
    _watcher: notify::RecommendedWatcher,
    pub rx: Receiver<PathBuf>,
}

/// Watch `root` recursively. Emits vault-relative paths of changed `.md` files
/// on the returned receiver. The caller decides when to call `Index::reindex_path`.
pub fn watch_vault(root: &Path) -> anyhow::Result<VaultWatcher> {
    let (tx, rx) = std::sync::mpsc::channel();
    // Canonicalize so the strip-prefix base matches the OS-reported event paths
    // (on macOS FSEvents reports /private/var... for a /var... tempdir, etc).
    let root_owned = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if let Ok(event) = res {
            for path in event.paths {
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(rel) = path.strip_prefix(&root_owned) {
                        let _ = tx.send(PathBuf::from(rel.to_string_lossy().replace('\\', "/")));
                    }
                }
            }
        }
    })?;
    watcher.watch(root, RecursiveMode::Recursive)?;
    Ok(VaultWatcher {
        _watcher: watcher,
        rx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn watcher_reports_a_new_md_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let w = watch_vault(root).unwrap();
        std::fs::write(root.join("fresh.md"), "# Fresh").unwrap();
        // notify is async; poll the channel briefly.
        let mut saw = false;
        for _ in 0..50 {
            if let Ok(p) = w.rx.recv_timeout(Duration::from_millis(100)) {
                if p == Path::new("fresh.md") {
                    saw = true;
                    break;
                }
            }
        }
        assert!(saw, "watcher did not report fresh.md");
    }
}
