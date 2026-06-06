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
}
