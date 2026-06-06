use crate::index::Index;
use crate::links::{rebase_relative, rewrite_inbound};
use crate::query;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenamePlan {
    pub from: String,
    pub to: String,
    pub files_changed: Vec<FileChange>,
    pub links_rewritten: usize,
    pub warnings: Vec<String>,
    pub applied: bool,
    /// path -> new content for rewritten referrers (not serialized).
    #[serde(skip)]
    pub writes: Vec<(String, String)>,
    /// (old_path, new_path, new_content) for the moved file (not serialized).
    #[serde(skip)]
    pub moved: Option<(String, String, String)>,
}

fn unified(path: &str, old: &str, new: &str) -> String {
    similar::TextDiff::from_lines(old, new)
        .unified_diff()
        .header(path, path)
        .to_string()
}

fn all_node_paths(idx: &Index) -> anyhow::Result<Vec<String>> {
    let mut stmt = idx.conn().prepare("SELECT path FROM nodes ORDER BY path")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Plan a rename: validate, rewrite inbound links in every referrer, rebase the
/// moved file's own relative links. Writes nothing.
pub fn plan_rename(idx: &Index, root: &Path, from: &str, to: &str) -> anyhow::Result<RenamePlan> {
    let all = all_node_paths(idx)?;
    if !all.iter().any(|p| p == from) {
        anyhow::bail!("source not found: {from}");
    }
    if all.iter().any(|p| p == to) || root.join(to).exists() {
        anyhow::bail!("target already exists: {to}");
    }

    let mut files_changed = Vec::new();
    let mut writes = Vec::new();
    let mut links_rewritten = 0usize;
    let warnings = Vec::new();

    // Referrers: every source with an inbound edge to `from` (skip self).
    for edge in query::backlinks(idx, from)? {
        let src = edge.src;
        if src == from {
            continue;
        }
        // A referrer may appear once per inbound edge; only process it once.
        if writes.iter().any(|(p, _)| p == &src) {
            continue;
        }
        let content = std::fs::read_to_string(root.join(&src))?;
        let (new_content, n) = rewrite_inbound(&content, &src, from, to, &all);
        if n > 0 && new_content != content {
            links_rewritten += n;
            files_changed.push(FileChange {
                path: src.clone(),
                diff: unified(&src, &content, &new_content),
            });
            writes.push((src.clone(), new_content));
        }
    }

    // The moved file: rebase its own relative links, then it moves.
    let from_content = std::fs::read_to_string(root.join(from))?;
    let (moved_content, _) = rebase_relative(&from_content, from, to, &all);
    files_changed.push(FileChange {
        path: format!("{from} -> {to}"),
        diff: unified(to, &from_content, &moved_content),
    });

    Ok(RenamePlan {
        from: from.to_string(),
        to: to.to_string(),
        files_changed,
        links_rewritten,
        warnings,
        applied: false,
        writes,
        moved: Some((from.to_string(), to.to_string(), moved_content)),
    })
}

/// Apply a rename plan: write rewritten referrers, move the file, reindex all
/// touched paths. Best-effort rollback of referrer writes on mid-write failure.
pub fn apply_rename(idx: &Index, root: &Path, plan: &RenamePlan) -> anyhow::Result<()> {
    let mut written: Vec<(String, String)> = Vec::new();
    let result = (|| -> anyhow::Result<()> {
        for (rel, new_content) in &plan.writes {
            let original = std::fs::read_to_string(root.join(rel)).unwrap_or_default();
            std::fs::write(root.join(rel), new_content)?;
            written.push((rel.clone(), original));
        }
        if let Some((from, to, moved_content)) = &plan.moved {
            if let Some(parent) = root.join(to).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(root.join(to), moved_content)?;
            std::fs::remove_file(root.join(from))?;
        }
        Ok(())
    })();

    if let Err(e) = result {
        for (rel, original) in &written {
            let _ = std::fs::write(root.join(rel), original);
        }
        return Err(e);
    }

    if let Some((from, to, _)) = &plan.moved {
        idx.reindex_path(root, from)?; // gone -> removed from index
        idx.reindex_path(root, to)?; // new file -> added
    }
    for (rel, _) in &plan.writes {
        idx.reindex_path(root, rel)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;
    use std::fs;

    fn vault() -> (tempfile::TempDir, Index, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/old.md"), "# Old\n\nbody\n").unwrap();
        fs::write(root.join("docs/refer.md"), "# Refer\n\nsee [[old]] now\n").unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(&root, ".aiignore").unwrap();
        (dir, idx, root)
    }

    #[test]
    fn plan_rename_rewrites_inbound_and_lists_changes() {
        let (_d, idx, root) = vault();
        let plan = plan_rename(&idx, &root, "docs/old.md", "docs/new.md").unwrap();
        assert_eq!(plan.from, "docs/old.md");
        assert_eq!(plan.to, "docs/new.md");
        assert_eq!(plan.links_rewritten, 1);
        let changed: Vec<&str> = plan.files_changed.iter().map(|c| c.path.as_str()).collect();
        assert!(changed.contains(&"docs/refer.md"));
        assert!(plan.files_changed.iter().any(|c| c.diff.contains("[[new]]")));
    }

    #[test]
    fn plan_rename_rejects_collision() {
        let (_d, idx, root) = vault();
        let err = plan_rename(&idx, &root, "docs/old.md", "docs/refer.md").unwrap_err();
        assert!(err.to_string().contains("exists"));
    }

    #[test]
    fn plan_rename_rejects_missing_source() {
        let (_d, idx, root) = vault();
        let err = plan_rename(&idx, &root, "docs/ghost.md", "docs/new.md").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn apply_rename_writes_moves_and_reindexes() {
        let (_d, idx, root) = vault();
        let plan = plan_rename(&idx, &root, "docs/old.md", "docs/new.md").unwrap();
        apply_rename(&idx, &root, &plan).unwrap();

        assert!(!root.join("docs/old.md").exists());
        assert!(root.join("docs/new.md").exists());
        let refer = std::fs::read_to_string(root.join("docs/refer.md")).unwrap();
        assert!(refer.contains("[[new]]"));
        assert!(idx.node_id("docs/old.md").unwrap().is_none());
        assert!(idx.node_id("docs/new.md").unwrap().is_some());
        let bl = query::backlinks(&idx, "docs/new.md").unwrap();
        assert!(bl.iter().any(|e| e.src == "docs/refer.md"));
        assert!(query::broken_links(&idx, None, 100).unwrap().is_empty());
    }
}
