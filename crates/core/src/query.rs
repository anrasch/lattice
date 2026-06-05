use crate::index::Index;
use crate::model::{Edge, EdgeKind, Node, NodeType};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MetaValue {
    pub value: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetaKey {
    pub key: String,
    pub distinct: usize,
    pub values: Vec<MetaValue>,
}

fn row_to_node(r: &rusqlite::Row) -> rusqlite::Result<Node> {
    let type_str: String = r.get(2)?;
    Ok(Node {
        path: r.get(0)?,
        title: r.get(1)?,
        node_type: match type_str.as_str() {
            "index" => NodeType::Index,
            _ => NodeType::Note,
        },
    })
}

fn edge_from_row(r: &rusqlite::Row) -> rusqlite::Result<Edge> {
    let kind_str: String = r.get(2)?;
    Ok(Edge {
        src: r.get(0)?,
        dst: r.get(1)?,
        kind: EdgeKind::from_tag(&kind_str).unwrap_or(EdgeKind::Wikilink),
        raw_target: r.get(3)?,
        resolved: r.get::<_, i64>(4)? != 0,
    })
}

const EDGE_SELECT: &str = "SELECT n1.path AS src, n2.path AS dst, e.kind, e.raw_target, e.resolved
    FROM edges e
    JOIN nodes n1 ON n1.id = e.src_id
    LEFT JOIN nodes n2 ON n2.id = e.dst_id";

/// Inbound edges to `note` (who links here).
pub fn backlinks(idx: &Index, note: &str) -> anyhow::Result<Vec<Edge>> {
    let sql = format!("{EDGE_SELECT} WHERE n2.path = ?1 ORDER BY src");
    let mut stmt = idx.conn().prepare(&sql)?;
    let rows = stmt.query_map(params![note], edge_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Outbound edges from `note`.
pub fn links(idx: &Index, note: &str) -> anyhow::Result<Vec<Edge>> {
    let sql = format!("{EDGE_SELECT} WHERE n1.path = ?1 ORDER BY dst");
    let mut stmt = idx.conn().prepare(&sql)?;
    let rows = stmt.query_map(params![note], edge_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Nodes with no resolved inbound edge of any kind. Optionally scoped to a
/// directory prefix and capped at `limit` (keeps an AI surface from drowning).
pub fn orphans(idx: &Index, under: Option<&str>, limit: usize) -> anyhow::Result<Vec<Node>> {
    let limit = limit as i64;
    let base = "SELECT path, title, type FROM nodes n
         WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.dst_id = n.id AND e.resolved = 1)";
    match under {
        Some(d) => {
            let prefix = format!("{}/%", d.trim_end_matches('/'));
            let mut stmt = idx
                .conn()
                .prepare(&format!("{base} AND n.path LIKE ?1 ORDER BY path LIMIT ?2"))?;
            let rows = stmt.query_map(params![prefix, limit], row_to_node)?;
            Ok(rows.collect::<Result<_, _>>()?)
        }
        None => {
            let mut stmt = idx
                .conn()
                .prepare(&format!("{base} ORDER BY path LIMIT ?1"))?;
            let rows = stmt.query_map(params![limit], row_to_node)?;
            Ok(rows.collect::<Result<_, _>>()?)
        }
    }
}

/// Unresolved link-like edges (broken `[[targets]]`). Optionally scoped to a
/// source-directory prefix and capped at `limit`.
pub fn broken_links(idx: &Index, under: Option<&str>, limit: usize) -> anyhow::Result<Vec<Edge>> {
    let limit = limit as i64;
    let base = format!(
        "{EDGE_SELECT} WHERE e.resolved = 0 AND e.kind IN ('wikilink','frontmatter_ref','supersedes')"
    );
    match under {
        Some(d) => {
            let prefix = format!("{}/%", d.trim_end_matches('/'));
            let mut stmt = idx.conn().prepare(&format!(
                "{base} AND n1.path LIKE ?1 ORDER BY src, raw_target LIMIT ?2"
            ))?;
            let rows = stmt.query_map(params![prefix, limit], edge_from_row)?;
            Ok(rows.collect::<Result<_, _>>()?)
        }
        None => {
            let mut stmt = idx
                .conn()
                .prepare(&format!("{base} ORDER BY src, raw_target LIMIT ?1"))?;
            let rows = stmt.query_map(params![limit], edge_from_row)?;
            Ok(rows.collect::<Result<_, _>>()?)
        }
    }
}

/// Nodes whose frontmatter matches ALL given key=value pairs (string compare),
/// optionally scoped to a directory prefix.
pub fn query(
    idx: &Index,
    filters: &[(&str, &str)],
    under: Option<&str>,
) -> anyhow::Result<Vec<Node>> {
    let prefix = under.map(|d| format!("{}/%", d.trim_end_matches('/')));

    if filters.is_empty() {
        return match &prefix {
            Some(p) => {
                let mut stmt = idx.conn().prepare(
                    "SELECT path, title, type FROM nodes WHERE path LIKE ?1 ORDER BY path",
                )?;
                let rows = stmt.query_map(params![p], row_to_node)?;
                Ok(rows.collect::<Result<_, _>>()?)
            }
            None => {
                let mut stmt = idx
                    .conn()
                    .prepare("SELECT path, title, type FROM nodes ORDER BY path")?;
                let rows = stmt.query_map([], row_to_node)?;
                Ok(rows.collect::<Result<_, _>>()?)
            }
        };
    }

    // INTERSECT one node_meta lookup per filter, then optionally prefix-filter.
    let mut clauses = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    for (k, v) in filters {
        clauses.push("SELECT node_id FROM node_meta WHERE key = ? AND value = ?".to_string());
        binds.push((*k).to_string());
        binds.push((*v).to_string());
    }
    let mut sql = format!(
        "SELECT path, title, type FROM nodes WHERE id IN ({})",
        clauses.join(" INTERSECT ")
    );
    if let Some(p) = &prefix {
        sql.push_str(" AND path LIKE ?");
        binds.push(p.clone());
    }
    sql.push_str(" ORDER BY path");
    let mut stmt = idx.conn().prepare(&sql)?;
    let bind_refs: Vec<&dyn rusqlite::ToSql> =
        binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(bind_refs.as_slice(), row_to_node)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Full-text search over title+body, ranked by bm25. `text` is an FTS5 query.
/// Optionally scoped to a directory prefix (drops cross-collection noise).
pub fn search(
    idx: &Index,
    text: &str,
    under: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<Node>> {
    let limit = limit as i64;
    match under {
        Some(d) => {
            let prefix = format!("{}/%", d.trim_end_matches('/'));
            let mut stmt = idx.conn().prepare(
                "SELECT n.path, n.title, n.type FROM fts JOIN nodes n ON n.id = fts.rowid
                 WHERE fts MATCH ?1 AND n.path LIKE ?2 ORDER BY bm25(fts) LIMIT ?3",
            )?;
            let rows = stmt.query_map(params![text, prefix, limit], row_to_node)?;
            Ok(rows.collect::<Result<_, _>>()?)
        }
        None => {
            let mut stmt = idx.conn().prepare(
                "SELECT n.path, n.title, n.type FROM fts JOIN nodes n ON n.id = fts.rowid
                 WHERE fts MATCH ?1 ORDER BY bm25(fts) LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![text, limit], row_to_node)?;
            Ok(rows.collect::<Result<_, _>>()?)
        }
    }
}

/// Nodes whose path is within `dir` (recursive), capped at `limit`. Root
/// (`""`, `"/"`, `"."`) spans the whole vault — pair with `dir_summary` for a
/// budget-friendly map first.
pub fn index_tree(idx: &Index, dir: &str, limit: usize) -> anyhow::Result<Vec<Node>> {
    let limit = limit as i64;
    let d = dir.trim().trim_matches('/');
    if d.is_empty() || d == "." {
        let mut stmt = idx
            .conn()
            .prepare("SELECT path, title, type FROM nodes ORDER BY path LIMIT ?1")?;
        let rows = stmt.query_map(params![limit], row_to_node)?;
        return Ok(rows.collect::<Result<_, _>>()?);
    }
    let prefix = format!("{d}/%");
    let mut stmt = idx
        .conn()
        .prepare("SELECT path, title, type FROM nodes WHERE path LIKE ?1 ORDER BY path LIMIT ?2")?;
    let rows = stmt.query_map(params![prefix, limit], row_to_node)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// A directory and how many notes live under it (recursively).
#[derive(Debug, Clone, Serialize)]
pub struct DirCount {
    pub dir: String,
    pub count: usize,
}

/// A budget-friendly map of the vault: every directory with its note count.
/// Cheap "shape of the vault" instead of dumping every leaf.
pub fn dir_summary(idx: &Index) -> anyhow::Result<Vec<DirCount>> {
    let mut stmt = idx.conn().prepare("SELECT path FROM nodes")?;
    let paths: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for p in &paths {
        // Count this note against every ancestor directory.
        let mut acc = String::new();
        let parts: Vec<&str> = p.split('/').collect();
        for seg in &parts[..parts.len().saturating_sub(1)] {
            acc = if acc.is_empty() {
                seg.to_string()
            } else {
                format!("{acc}/{seg}")
            };
            *counts.entry(acc.clone()).or_insert(0) += 1;
        }
        *counts.entry(String::new()).or_insert(0) += 1; // root total
    }
    Ok(counts
        .into_iter()
        .map(|(dir, count)| DirCount {
            dir: if dir.is_empty() { "/".to_string() } else { dir },
            count,
        })
        .collect())
}

/// Notes whose `updated` (or `date`) frontmatter is on/after `since` (ISO date
/// string compare), newest first. For re-grounding a session on just the deltas.
pub fn changed_since(idx: &Index, since: &str, limit: usize) -> anyhow::Result<Vec<Node>> {
    let mut stmt = idx.conn().prepare(
        "SELECT DISTINCT n.path, n.title, n.type, m.value AS updated
         FROM nodes n JOIN node_meta m ON m.node_id = n.id
         WHERE m.key IN ('updated','date') AND m.value >= ?1
         ORDER BY updated DESC, n.path LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![since, limit as i64], row_to_node)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Resolved supersession edges: `src` supersedes (replaces) `dst`. A `dst`
/// appearing here is an overruled decision — read the superseding note instead.
pub fn superseded(idx: &Index, limit: usize) -> anyhow::Result<Vec<Edge>> {
    let sql = format!(
        "{EDGE_SELECT} WHERE e.kind = 'supersedes' AND e.resolved = 1 ORDER BY src LIMIT ?1"
    );
    let mut stmt = idx.conn().prepare(&sql)?;
    let rows = stmt.query_map(params![limit as i64], edge_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    }
}

/// Enumerate frontmatter keys with their values + counts, for filter discovery.
/// Values per key are capped at 40 (ordered by frequency) and truncated to 80
/// chars so free-text frontmatter doesn't drown the categorical keys.
pub fn meta_keys(idx: &Index) -> anyhow::Result<Vec<MetaKey>> {
    let mut stmt = idx.conn().prepare(
        "SELECT key, value, COUNT(*) c FROM node_meta GROUP BY key, value ORDER BY key, c DESC, value",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, i64>(2)?,
        ))
    })?;
    let mut out: Vec<MetaKey> = Vec::new();
    for row in rows {
        let (key, value, count) = row?;
        match out.last_mut() {
            Some(mk) if mk.key == key => {
                mk.distinct += 1;
                if mk.values.len() < 40 {
                    mk.values.push(MetaValue {
                        value: truncate(&value, 80),
                        count,
                    });
                }
            }
            _ => out.push(MetaKey {
                key,
                distinct: 1,
                values: vec![MetaValue {
                    value: truncate(&value, 80),
                    count,
                }],
            }),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;
    use std::fs;

    fn built() -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/README.md"), "# Docs\n\n[[guide]]\n").unwrap();
        fs::write(root.join("docs/guide.md"), "# Guide\n\n[[missing]]\n").unwrap();
        fs::write(
            root.join("orphan.md"),
            "# Orphan\n\nno links in or out that resolve\n",
        )
        .unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();
        (dir, idx)
    }

    #[test]
    fn backlinks_lists_inbound() {
        let (_d, idx) = built();
        let b = backlinks(&idx, "docs/guide.md").unwrap();
        assert!(b
            .iter()
            .any(|e| e.src == "docs/README.md" && e.kind == EdgeKind::Wikilink));
    }

    #[test]
    fn links_lists_outbound_resolved() {
        let (_d, idx) = built();
        let l = links(&idx, "docs/README.md").unwrap();
        assert!(l.iter().any(|e| e.dst.as_deref() == Some("docs/guide.md")));
    }

    #[test]
    fn orphans_have_no_inbound() {
        let (_d, idx) = built();
        let o = orphans(&idx, None, 1000).unwrap();
        assert!(o.iter().any(|n| n.path == "orphan.md"));
        assert!(!o.iter().any(|n| n.path == "docs/guide.md"));
    }

    #[test]
    fn broken_links_lists_unresolved() {
        let (_d, idx) = built();
        let bl = broken_links(&idx, None, 1000).unwrap();
        assert!(bl
            .iter()
            .any(|e| e.raw_target == "missing" && e.src == "docs/guide.md"));
    }

    fn built_meta() -> (tempfile::TempDir, Index) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(
            root.join("a.md"),
            "---\ntype: spec\nstatus: active\n---\n# A\n\nalpha apple\n",
        )
        .unwrap();
        fs::write(
            root.join("b.md"),
            "---\ntype: spec\nstatus: done\n---\n# B\n\nbeta\n",
        )
        .unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();
        (dir, idx)
    }

    #[test]
    fn query_filters_by_frontmatter() {
        let (_d, idx) = built_meta();
        let r = query(&idx, &[("type", "spec"), ("status", "active")], None).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].path, "a.md");
    }

    #[test]
    fn search_matches_body_text() {
        let (_d, idx) = built_meta();
        let hits = search(&idx, "apple", None, 10).unwrap();
        assert_eq!(hits[0].path, "a.md");
    }

    #[test]
    fn meta_keys_enumerates_frontmatter_with_counts() {
        let (_d, idx) = built_meta();
        let keys = meta_keys(&idx).unwrap();
        let type_key = keys.iter().find(|k| k.key == "type").unwrap();
        // both a.md and b.md have type: spec
        assert_eq!(
            type_key
                .values
                .iter()
                .find(|v| v.value == "spec")
                .unwrap()
                .count,
            2
        );
        let status = keys.iter().find(|k| k.key == "status").unwrap();
        assert_eq!(status.distinct, 2); // active + done
    }

    #[test]
    fn index_tree_root_returns_whole_vault() {
        let (_d, idx) = built_meta();
        let all = index_tree(&idx, "/", 1000).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(index_tree(&idx, "", 1000).unwrap().len(), 2);
    }

    #[test]
    fn index_tree_lists_nodes_under_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs/sub")).unwrap();
        fs::write(root.join("docs/README.md"), "# Docs").unwrap();
        fs::write(root.join("docs/one.md"), "# One").unwrap();
        fs::write(root.join("docs/sub/two.md"), "# Two").unwrap();
        fs::write(root.join("top.md"), "# Top").unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();

        let mut paths: Vec<String> = index_tree(&idx, "docs", 1000)
            .unwrap()
            .into_iter()
            .map(|n| n.path)
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            vec!["docs/README.md", "docs/one.md", "docs/sub/two.md"]
        );
        assert!(!paths.contains(&"top.md".to_string()));
    }

    #[test]
    fn supersedes_edge_and_superseded_query() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("old.md"), "# Old\n").unwrap();
        fs::write(root.join("new.md"), "---\nsupersedes: [old]\n---\n# New\n").unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();

        let s = superseded(&idx, 100).unwrap();
        assert!(s.iter().any(|e| e.src == "new.md"
            && e.dst.as_deref() == Some("old.md")
            && e.kind == EdgeKind::Supersedes));
        let bl = backlinks(&idx, "old.md").unwrap();
        assert!(bl
            .iter()
            .any(|e| e.kind == EdgeKind::Supersedes && e.src == "new.md"));
    }

    #[test]
    fn changed_since_filters_by_date() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(
            root.join("old.md"),
            "---\nupdated: 2026-01-01\n---\n# Old\n",
        )
        .unwrap();
        fs::write(
            root.join("new.md"),
            "---\nupdated: 2026-06-01\n---\n# New\n",
        )
        .unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();

        let c = changed_since(&idx, "2026-05-01", 100).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].path, "new.md");
    }

    #[test]
    fn dir_summary_counts_recursively() {
        let (_d, idx) = built(); // docs/README.md, docs/guide.md, orphan.md
        let s = dir_summary(&idx).unwrap();
        assert_eq!(s.iter().find(|d| d.dir == "/").unwrap().count, 3);
        assert_eq!(s.iter().find(|d| d.dir == "docs").unwrap().count, 2);
    }

    #[test]
    fn search_scopes_by_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/spec.md"), "# Spec\n\nwidget term\n").unwrap();
        fs::write(root.join("fiction.md"), "# Fiction\n\nwidget term\n").unwrap();
        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();

        let scoped = search(&idx, "widget", Some("docs"), 10).unwrap();
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].path, "docs/spec.md");
        assert_eq!(search(&idx, "widget", None, 10).unwrap().len(), 2);
    }
}
