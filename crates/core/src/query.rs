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
    let base =
        format!("{EDGE_SELECT} WHERE e.resolved = 0 AND e.kind IN ('wikilink','frontmatter_ref')");
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

/// Nodes whose frontmatter matches ALL given key=value pairs (string compare).
pub fn query(idx: &Index, filters: &[(&str, &str)]) -> anyhow::Result<Vec<Node>> {
    if filters.is_empty() {
        let mut stmt = idx
            .conn()
            .prepare("SELECT path, title, type FROM nodes ORDER BY path")?;
        let rows = stmt.query_map([], row_to_node)?;
        return Ok(rows.collect::<Result<_, _>>()?);
    }
    // INTERSECT one node_meta lookup per filter.
    let mut clauses = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    for (k, v) in filters {
        clauses.push("SELECT node_id FROM node_meta WHERE key = ? AND value = ?".to_string());
        binds.push((*k).to_string());
        binds.push((*v).to_string());
    }
    let sql = format!(
        "SELECT path, title, type FROM nodes WHERE id IN ({}) ORDER BY path",
        clauses.join(" INTERSECT ")
    );
    let mut stmt = idx.conn().prepare(&sql)?;
    let bind_refs: Vec<&dyn rusqlite::ToSql> =
        binds.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(bind_refs.as_slice(), row_to_node)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Full-text search over title+body, ranked by bm25. `text` is an FTS5 query.
pub fn search(idx: &Index, text: &str, limit: usize) -> anyhow::Result<Vec<Node>> {
    let mut stmt = idx.conn().prepare(
        "SELECT n.path, n.title, n.type
         FROM fts JOIN nodes n ON n.id = fts.rowid
         WHERE fts MATCH ?1
         ORDER BY bm25(fts) LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![text, limit as i64], row_to_node)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// All nodes whose path is within `dir` (recursive), i.e. the `contains` tree.
/// Root (`""`, `"/"`, or `"."`) returns the whole vault.
pub fn index_tree(idx: &Index, dir: &str) -> anyhow::Result<Vec<Node>> {
    let d = dir.trim().trim_matches('/');
    if d.is_empty() || d == "." {
        let mut stmt = idx
            .conn()
            .prepare("SELECT path, title, type FROM nodes ORDER BY path")?;
        let rows = stmt.query_map([], row_to_node)?;
        return Ok(rows.collect::<Result<_, _>>()?);
    }
    let prefix = format!("{d}/%");
    let mut stmt = idx
        .conn()
        .prepare("SELECT path, title, type FROM nodes WHERE path LIKE ?1 ORDER BY path")?;
    let rows = stmt.query_map(params![prefix], row_to_node)?;
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
        let r = query(&idx, &[("type", "spec"), ("status", "active")]).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].path, "a.md");
    }

    #[test]
    fn search_matches_body_text() {
        let (_d, idx) = built_meta();
        let hits = search(&idx, "apple", 10).unwrap();
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
        let all = index_tree(&idx, "/").unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(index_tree(&idx, "").unwrap().len(), 2);
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

        let mut paths: Vec<String> = index_tree(&idx, "docs")
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
}
