use crate::index::Index;
use crate::links::{rebase_relative, rewrite_inbound};
use crate::query;
use crate::resolve::{resolve_target, Resolution};
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

/// Split a doc into (frontmatter_lines, body) excluding the `---` fences.
/// None if there is no leading block.
fn split_block(content: &str) -> Option<(Vec<String>, String)> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let block = rest[..end].lines().map(|l| l.to_string()).collect();
    let body = rest[end + 5..].to_string();
    Some((block, body))
}

fn key_of(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let colon = trimmed.find(':')?;
    Some(trimmed[..colon].trim())
}

/// Append a value into an inline list line like `supersedes: [a, b]` (de-duped).
fn append_to_list(line: &str, value: &str) -> String {
    let (key, rhs) = line.split_once(':').unwrap();
    let rhs = rhs.trim();
    let inner = rhs
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or("");
    let mut items: Vec<String> = inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !items.iter().any(|i| i == value) {
        items.push(value.to_string());
    }
    format!("{key}: [{}]", items.join(", "))
}

/// True if `key` is a block-style entry (empty inline value followed by an
/// indented child line — a `- item` list or nested map). The line-based patcher
/// only safely handles inline values / `[a, b]` lists, so these must be refused.
fn is_block_structured(block: &[String], key: &str) -> bool {
    for (i, line) in block.iter().enumerate() {
        if key_of(line) == Some(key) {
            let val = line.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
            if !val.is_empty() {
                return false; // inline scalar or [list]
            }
            if let Some(next) = block.get(i + 1) {
                return next.len() != next.trim_start().len(); // indented child
            }
            return false;
        }
    }
    false
}

/// Surgically patch the frontmatter block, preserving untouched lines. Creates a
/// block if absent. Returns (new_content, warnings). Refuses (returns content
/// unchanged + a warning) when it can't edit safely: an unparseable existing
/// block (CRLF / missing closing fence) or a touched key that is block-style.
pub fn patch_frontmatter_text(
    content: &str,
    set: &[(String, String)],
    add: &[(String, Vec<String>)],
    unset: &[String],
) -> (String, Vec<String>) {
    // Refuse an existing-but-unparseable block rather than prepend a second one.
    let starts_fm = content
        .strip_prefix('\u{feff}')
        .unwrap_or(content)
        .starts_with("---");
    let parsed = split_block(content);
    if starts_fm && parsed.is_none() {
        return (
            content.to_string(),
            vec!["refused: cannot parse the frontmatter block (CRLF line endings or missing closing `---`); not patched to avoid corruption".to_string()],
        );
    }

    let (mut block, body, had_block) = match parsed {
        Some((b, body)) => (b, body, true),
        None => (Vec::new(), content.to_string(), false),
    };

    // Refuse if any key we'd touch is block-style (would orphan its child lines).
    let touched = set
        .iter()
        .map(|(k, _)| k.as_str())
        .chain(add.iter().map(|(k, _)| k.as_str()))
        .chain(unset.iter().map(|k| k.as_str()));
    for key in touched {
        if is_block_structured(&block, key) {
            return (
                content.to_string(),
                vec![format!(
                    "refused: frontmatter key `{key}` is block-style (multi-line); the patcher only supports inline `[a, b]` lists — not patched to avoid corruption"
                )],
            );
        }
    }

    let warnings = Vec::new();

    block.retain(|line| !unset.iter().any(|k| key_of(line) == Some(k.as_str())));

    for (k, v) in set {
        if let Some(line) = block.iter_mut().find(|l| key_of(l) == Some(k.as_str())) {
            *line = format!("{k}: {v}");
        } else {
            block.push(format!("{k}: {v}"));
        }
    }

    for (k, values) in add {
        if let Some(line) = block.iter_mut().find(|l| key_of(l) == Some(k.as_str())) {
            for v in values {
                *line = append_to_list(line, v);
            }
        } else {
            block.push(format!("{k}: [{}]", values.join(", ")));
        }
    }

    if block.is_empty() && !had_block {
        return (content.to_string(), warnings);
    }

    let mut out = String::from("---\n");
    for line in &block {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("---\n");
    out.push_str(&body);
    (out, warnings)
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchPlan {
    pub note: String,
    pub diff: String,
    pub warnings: Vec<String>,
    pub applied: bool,
    #[serde(skip)]
    pub new_content: String,
}

/// Plan a frontmatter patch: compute the new content + warn on dangling refs.
pub fn plan_patch(
    idx: &Index,
    root: &Path,
    note: &str,
    set: &[(String, String)],
    add: &[(String, Vec<String>)],
    unset: &[String],
) -> anyhow::Result<PatchPlan> {
    let all = all_node_paths(idx)?;
    if !all.iter().any(|p| p == note) {
        anyhow::bail!("note not found: {note}");
    }
    let content = std::fs::read_to_string(root.join(note))?;
    let (new_content, mut warnings) = patch_frontmatter_text(&content, set, add, unset);

    let ref_keys = ["related", "supersedes"];
    for (k, v) in set {
        if ref_keys.contains(&k.as_str())
            && matches!(resolve_target(note, v, &all), Resolution::NotFound)
        {
            warnings.push(format!("dangling reference: {k} -> {v} (no such note)"));
        }
    }
    for (k, values) in add {
        if ref_keys.contains(&k.as_str()) {
            for v in values {
                if matches!(resolve_target(note, v, &all), Resolution::NotFound) {
                    warnings.push(format!("dangling reference: {k} -> {v} (no such note)"));
                }
            }
        }
    }

    Ok(PatchPlan {
        note: note.to_string(),
        diff: unified(note, &content, &new_content),
        warnings,
        applied: false,
        new_content,
    })
}

/// Apply a patch plan: write the note + reindex it.
pub fn apply_patch(idx: &Index, root: &Path, plan: &PatchPlan) -> anyhow::Result<()> {
    std::fs::write(root.join(&plan.note), &plan.new_content)?;
    idx.reindex_path(root, &plan.note)?;
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
        assert!(plan
            .files_changed
            .iter()
            .any(|c| c.diff.contains("[[new]]")));
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

    #[test]
    fn patch_sets_replaces_value_preserving_other_keys() {
        let doc = "---\ntype: spec\nstatus: active\n---\n# T\n";
        let set = vec![("status".to_string(), "shipped".to_string())];
        let (out, _) = patch_frontmatter_text(doc, &set, &[], &[]);
        assert_eq!(out, "---\ntype: spec\nstatus: shipped\n---\n# T\n");
    }

    #[test]
    fn patch_set_inserts_missing_key() {
        let doc = "---\ntype: spec\n---\n# T\n";
        let set = vec![("updated".to_string(), "2026-06-06".to_string())];
        let (out, _) = patch_frontmatter_text(doc, &set, &[], &[]);
        assert_eq!(out, "---\ntype: spec\nupdated: 2026-06-06\n---\n# T\n");
    }

    #[test]
    fn patch_add_appends_to_list_field() {
        let doc = "---\nsupersedes: [a]\n---\n# T\n";
        let add = vec![("supersedes".to_string(), vec!["b".to_string()])];
        let (out, _) = patch_frontmatter_text(doc, &[], &add, &[]);
        assert_eq!(out, "---\nsupersedes: [a, b]\n---\n# T\n");
    }

    #[test]
    fn patch_add_creates_list_when_absent() {
        let doc = "---\ntype: spec\n---\n# T\n";
        let add = vec![("supersedes".to_string(), vec!["old".to_string()])];
        let (out, _) = patch_frontmatter_text(doc, &[], &add, &[]);
        assert_eq!(out, "---\ntype: spec\nsupersedes: [old]\n---\n# T\n");
    }

    #[test]
    fn patch_unset_removes_key() {
        let doc = "---\ntype: spec\nstatus: active\n---\n# T\n";
        let (out, _) = patch_frontmatter_text(doc, &[], &[], &["status".to_string()]);
        assert_eq!(out, "---\ntype: spec\n---\n# T\n");
    }

    #[test]
    fn patch_creates_block_when_absent() {
        let doc = "# Title\n\nbody\n";
        let set = vec![("status".to_string(), "active".to_string())];
        let (out, _) = patch_frontmatter_text(doc, &set, &[], &[]);
        assert_eq!(out, "---\nstatus: active\n---\n# Title\n\nbody\n");
    }

    #[test]
    fn patch_refuses_block_style_list_unchanged() {
        let doc = "---\ntags:\n  - a\n  - b\n---\n# T\n";
        let add = vec![("tags".to_string(), vec!["c".to_string()])];
        let (out, w) = patch_frontmatter_text(doc, &[], &add, &[]);
        assert_eq!(out, doc); // untouched — no corruption
        assert!(w
            .iter()
            .any(|m| m.contains("refused") && m.contains("block-style")));
    }

    #[test]
    fn patch_refuses_unparseable_crlf_block_unchanged() {
        let doc = "---\r\nstatus: active\r\n---\r\n# T\r\n";
        let set = vec![("status".to_string(), "shipped".to_string())];
        let (out, w) = patch_frontmatter_text(doc, &set, &[], &[]);
        assert_eq!(out, doc); // untouched — no second block prepended
        assert!(w.iter().any(|m| m.contains("refused")));
    }

    #[test]
    fn patch_still_works_on_untouched_block_keys_nearby() {
        // a block-style key exists, but we only touch a different inline key
        let doc = "---\nstatus: active\ntags:\n  - a\n---\n# T\n";
        let set = vec![("status".to_string(), "shipped".to_string())];
        let (out, w) = patch_frontmatter_text(doc, &set, &[], &[]);
        assert_eq!(out, "---\nstatus: shipped\ntags:\n  - a\n---\n# T\n");
        assert!(w.is_empty());
    }

    #[test]
    fn plan_and_apply_patch_updates_index() {
        let (_d, idx, root) = vault();
        // give docs/old.md frontmatter to patch
        fs::write(
            root.join("docs/old.md"),
            "---\nstatus: active\n---\n# Old\n",
        )
        .unwrap();
        idx.reindex_path(&root, "docs/old.md").unwrap();

        let set = vec![("status".to_string(), "shipped".to_string())];
        let plan = plan_patch(&idx, &root, "docs/old.md", &set, &[], &[]).unwrap();
        assert!(plan.diff.contains("status: shipped"));
        apply_patch(&idx, &root, &plan).unwrap();
        let r = query::query(&idx, &[("status", "shipped")], None).unwrap();
        assert!(r.iter().any(|n| n.path == "docs/old.md"));
    }

    #[test]
    fn plan_patch_warns_on_dangling_ref() {
        let (_d, idx, root) = vault();
        let add = vec![("supersedes".to_string(), vec!["ghost".to_string()])];
        let plan = plan_patch(&idx, &root, "docs/old.md", &[], &add, &[]).unwrap();
        assert!(plan.warnings.iter().any(|w| w.contains("ghost")));
    }
}
