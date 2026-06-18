//! Cross-process control channel: a short-lived writer (MCP/CLI) asks the
//! long-lived desktop app to open a note. The request is a tiny JSON file in
//! `~/.lattice` (outside any vault); the app watches the dir and acts on it.

use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

/// A request to open + focus `note` in the desktop app for `vault`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRequest {
    /// Canonicalized vault root the request is for.
    pub vault: String,
    /// Vault-relative note path to open.
    pub note: String,
    /// Unix millis when written (freshness + replay guard).
    pub ts: i64,
}

/// `~/.lattice` — the control directory, outside any vault.
pub fn control_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".lattice")
}

fn request_path_in(dir: &Path) -> PathBuf {
    dir.join("open-request.json")
}

/// Path of the open-request file under the real control dir.
pub fn open_request_path() -> PathBuf {
    request_path_in(&control_dir())
}

/// Write an open request for `note` in `vault_root` to the control dir.
pub fn write_open_request(vault_root: &Path, note: &str) -> anyhow::Result<()> {
    write_open_request_to(&control_dir(), vault_root, note)
}

fn write_open_request_to(dir: &Path, vault_root: &Path, note: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let vault = std::fs::canonicalize(vault_root)
        .unwrap_or_else(|_| vault_root.to_path_buf())
        .to_string_lossy()
        .to_string();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let req = OpenRequest {
        vault,
        note: note.to_string(),
        ts,
    };
    // Atomic: write a temp sibling then rename, so the watcher never reads a
    // half-written file.
    let tmp = dir.join("open-request.json.tmp");
    std::fs::write(&tmp, serde_json::to_string(&req)?)?;
    std::fs::rename(&tmp, request_path_in(dir))?;
    Ok(())
}

/// Read the current open request, or `None` if the file is absent.
pub fn read_open_request() -> anyhow::Result<Option<OpenRequest>> {
    read_open_request_from(&control_dir())
}

fn read_open_request_from(dir: &Path) -> anyhow::Result<Option<OpenRequest>> {
    match std::fs::read_to_string(request_path_in(dir)) {
        Ok(s) => Ok(Some(serde_json::from_str(&s)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// A running control-dir watcher. Drop to stop. Emits `()` on any change.
pub struct ControlWatcher {
    _watcher: notify::RecommendedWatcher,
    pub rx: Receiver<()>,
}

/// Watch the real control dir for open requests.
pub fn watch_control() -> anyhow::Result<ControlWatcher> {
    watch_control_dir(&control_dir())
}

fn watch_control_dir(dir: &Path) -> anyhow::Result<ControlWatcher> {
    std::fs::create_dir_all(dir)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok(ControlWatcher {
        _watcher: watcher,
        rx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn write_then_read_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        write_open_request_to(dir.path(), vault.path(), "docs/a.md").unwrap();
        let req = read_open_request_from(dir.path()).unwrap().unwrap();
        assert_eq!(req.note, "docs/a.md");
        let leaf = vault
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            req.vault.ends_with(&leaf),
            "vault path stored: {}",
            req.vault
        );
        assert!(req.ts > 0);
    }

    #[test]
    fn read_is_none_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_open_request_from(dir.path()).unwrap().is_none());
    }

    #[test]
    fn write_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        write_open_request_to(dir.path(), vault.path(), "a.md").unwrap();
        let names: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["open-request.json".to_string()]);
    }

    #[test]
    fn control_watcher_signals_on_write() {
        let dir = tempfile::tempdir().unwrap();
        let vault = tempfile::tempdir().unwrap();
        let w = watch_control_dir(dir.path()).unwrap();
        write_open_request_to(dir.path(), vault.path(), "a.md").unwrap();
        let mut saw = false;
        for _ in 0..50 {
            if w.rx.recv_timeout(Duration::from_millis(100)).is_ok() {
                saw = true;
                break;
            }
        }
        assert!(saw, "watcher did not signal on write");
    }
}
