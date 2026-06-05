use crate::index::Index;
use crate::model::{Edge, EdgeKind, Node, NodeType};
use rusqlite::params;

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
        kind: EdgeKind::from_str(&kind_str).unwrap_or(EdgeKind::Wikilink),
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

/// Nodes with no resolved inbound edge of any kind.
pub fn orphans(idx: &Index) -> anyhow::Result<Vec<Node>> {
    let mut stmt = idx.conn().prepare(
        "SELECT path, title, type FROM nodes n
         WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.dst_id = n.id AND e.resolved = 1)
         ORDER BY path",
    )?;
    let rows = stmt.query_map([], row_to_node)?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Unresolved link-like edges (broken `[[targets]]`).
pub fn broken_links(idx: &Index) -> anyhow::Result<Vec<Edge>> {
    let sql = format!(
        "{EDGE_SELECT} WHERE e.resolved = 0 AND e.kind IN ('wikilink','frontmatter_ref') ORDER BY src, raw_target"
    );
    let mut stmt = idx.conn().prepare(&sql)?;
    let rows = stmt.query_map([], edge_from_row)?;
    Ok(rows.collect::<Result<_, _>>()?)
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
        let o = orphans(&idx).unwrap();
        assert!(o.iter().any(|n| n.path == "orphan.md"));
        assert!(!o.iter().any(|n| n.path == "docs/guide.md"));
    }

    #[test]
    fn broken_links_lists_unresolved() {
        let (_d, idx) = built();
        let bl = broken_links(&idx).unwrap();
        assert!(bl
            .iter()
            .any(|e| e.raw_target == "missing" && e.src == "docs/guide.md"));
    }
}
