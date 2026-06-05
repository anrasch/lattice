use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct RawNote {
    pub content: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum WriteOutcome {
    /// Saved; new hash of the written bytes.
    Written { hash: String },
    /// File changed on disk since load; not overwritten. Caller resolves.
    Conflict { on_disk: String },
}

fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Read a note's raw bytes + content hash (the hash the editor must echo back).
pub fn read_raw(root: &Path, rel: &str) -> anyhow::Result<RawNote> {
    let bytes = std::fs::read(root.join(rel))?;
    let hash = hash_bytes(&bytes);
    Ok(RawNote {
        content: String::from_utf8_lossy(&bytes).to_string(),
        hash,
    })
}

/// Write `content` to `rel` only if the file's current on-disk hash equals
/// `expected_hash` (last-write-wins guard). A new file (nothing on disk) is
/// allowed when `expected_hash` is empty. Returns Conflict without writing if
/// the file changed underneath.
pub fn write_note(
    root: &Path,
    rel: &str,
    content: &str,
    expected_hash: &str,
) -> anyhow::Result<WriteOutcome> {
    let full = root.join(rel);
    let current = std::fs::read(&full).ok();
    match &current {
        Some(bytes) => {
            let on_disk_hash = hash_bytes(bytes);
            if on_disk_hash != expected_hash {
                return Ok(WriteOutcome::Conflict {
                    on_disk: String::from_utf8_lossy(bytes).to_string(),
                });
            }
        }
        None => {
            // New file: only allowed if the editor also thought it was new.
            if !expected_hash.is_empty() {
                return Ok(WriteOutcome::Conflict {
                    on_disk: String::new(),
                });
            }
        }
    }
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full, content.as_bytes())?;
    Ok(WriteOutcome::Written {
        hash: hash_bytes(content.as_bytes()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_raw_returns_content_and_hash() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("n.md"), "# Hi\n").unwrap();
        let r = read_raw(dir.path(), "n.md").unwrap();
        assert_eq!(r.content, "# Hi\n");
        assert_eq!(r.hash.len(), 64);
    }

    #[test]
    fn write_succeeds_when_expected_hash_matches() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("n.md"), "old\n").unwrap();
        let before = read_raw(dir.path(), "n.md").unwrap();
        let out = write_note(dir.path(), "n.md", "new\n", &before.hash).unwrap();
        assert!(matches!(out, WriteOutcome::Written { .. }));
        assert_eq!(
            fs::read_to_string(dir.path().join("n.md")).unwrap(),
            "new\n"
        );
    }

    #[test]
    fn write_conflicts_when_file_changed_underneath() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("n.md"), "v1\n").unwrap();
        let stale = read_raw(dir.path(), "n.md").unwrap();
        // someone else writes between load and save
        fs::write(dir.path().join("n.md"), "v2-external\n").unwrap();
        let out = write_note(dir.path(), "n.md", "my-edit\n", &stale.hash).unwrap();
        match out {
            WriteOutcome::Conflict { on_disk } => assert_eq!(on_disk, "v2-external\n"),
            _ => panic!("expected Conflict"),
        }
        // file NOT overwritten
        assert_eq!(
            fs::read_to_string(dir.path().join("n.md")).unwrap(),
            "v2-external\n"
        );
    }
}
