use rusqlite::Connection;
use std::path::Path;

pub struct Index {
    conn: Connection,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS nodes (
    id               INTEGER PRIMARY KEY,
    path             TEXT NOT NULL UNIQUE,
    title            TEXT NOT NULL,
    type             TEXT NOT NULL,
    mtime            INTEGER NOT NULL,
    frontmatter_json TEXT NOT NULL,
    body_hash        TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS node_meta (
    node_id INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    key     TEXT NOT NULL,
    value   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_node_meta ON node_meta(key, value);
CREATE TABLE IF NOT EXISTS edges (
    src_id     INTEGER NOT NULL REFERENCES nodes(id) ON DELETE CASCADE,
    dst_id     INTEGER REFERENCES nodes(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    raw_target TEXT NOT NULL,
    anchor     TEXT,
    resolved   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_edges_src ON edges(src_id);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON edges(dst_id);
CREATE VIRTUAL TABLE IF NOT EXISTS fts USING fts5(title, body);
"#;

impl Index {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> anyhow::Result<Self> {
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Index { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Insert or replace a note and its meta/edges/FTS rows. Edges are written
    /// unresolved (dst_id NULL, resolved 0); call `resolve_edges` afterward.
    pub fn upsert_note(&self, note: &crate::model::ParsedNote, mtime: i64) -> anyhow::Result<()> {
        let conn = &self.conn;
        // Remove any prior version first (cascades meta/edges; FTS handled in remove_note).
        self.remove_note(&note.rel_path)?;

        conn.execute(
            "INSERT INTO nodes(path, title, type, mtime, frontmatter_json, body_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                note.rel_path,
                note.title,
                node_type_str(note.node_type),
                mtime,
                serde_json::to_string(&note.frontmatter)?,
                note.body_hash,
            ],
        )?;
        let node_id = conn.last_insert_rowid();

        // Exploded frontmatter (scalar values only) into node_meta.
        if let Some(map) = note.frontmatter.as_object() {
            for (k, v) in map {
                if let Some(s) = scalar_to_string(v) {
                    conn.execute(
                        "INSERT INTO node_meta(node_id, key, value) VALUES (?1, ?2, ?3)",
                        rusqlite::params![node_id, k, s],
                    )?;
                }
            }
        }

        // FTS row keyed by node id.
        conn.execute(
            "INSERT INTO fts(rowid, title, body) VALUES (?1, ?2, ?3)",
            rusqlite::params![node_id, note.title, note.body_text],
        )?;

        // Edges: wikilinks + markdown links (kind=wikilink), frontmatter refs.
        for wl in &note.wikilinks {
            self.insert_unresolved_edge(node_id, "wikilink", &wl.target, wl.anchor.as_deref())?;
        }
        for t in &note.md_link_targets {
            let target = t.strip_suffix(".md").unwrap_or(t);
            self.insert_unresolved_edge(node_id, "wikilink", target, None)?;
        }
        for r in &note.frontmatter_refs {
            self.insert_unresolved_edge(node_id, "frontmatter_ref", r, None)?;
        }
        Ok(())
    }

    fn insert_unresolved_edge(
        &self,
        src_id: i64,
        kind: &str,
        raw_target: &str,
        anchor: Option<&str>,
    ) -> anyhow::Result<()> {
        self.conn.execute(
            "INSERT INTO edges(src_id, dst_id, kind, raw_target, anchor, resolved)
             VALUES (?1, NULL, ?2, ?3, ?4, 0)",
            rusqlite::params![src_id, kind, raw_target, anchor],
        )?;
        Ok(())
    }

    /// Remove a note by path (cascades meta/edges; FTS row removed explicitly).
    pub fn remove_note(&self, rel_path: &str) -> anyhow::Result<()> {
        if let Some(id) = self.node_id(rel_path)? {
            self.conn
                .execute("DELETE FROM fts WHERE rowid=?1", rusqlite::params![id])?;
            self.conn
                .execute("DELETE FROM nodes WHERE id=?1", rusqlite::params![id])?;
        }
        Ok(())
    }

    pub fn node_id(&self, rel_path: &str) -> anyhow::Result<Option<i64>> {
        let id = self
            .conn
            .query_row(
                "SELECT id FROM nodes WHERE path=?1",
                rusqlite::params![rel_path],
                |r| r.get(0),
            )
            .ok();
        Ok(id)
    }

    /// Full build: walk the vault, upsert every note, then resolve edges and add
    /// directory `contains` edges. Idempotent against an existing index.
    pub fn build(&self, root: &std::path::Path, ignore_file: &str) -> anyhow::Result<()> {
        let paths = crate::walk::walk_vault(root, ignore_file)?;
        for rel in &paths {
            let full = root.join(rel);
            let bytes = std::fs::read(&full)?;
            let mtime = file_mtime(&full);
            let note = crate::parse::parse_note(rel, &bytes);
            self.upsert_note(&note, mtime)?;
        }
        self.resolve_edges()?;
        self.add_contains_edges()?;
        Ok(())
    }

    /// Resolve every unresolved wikilink/frontmatter_ref edge against the current
    /// node set using the v1 rule. Ambiguous/not-found stay resolved=0.
    pub fn resolve_edges(&self) -> anyhow::Result<()> {
        let all_paths: Vec<String> = {
            let mut stmt = self.conn.prepare("SELECT path FROM nodes")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        let pending: Vec<(i64, String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT e.rowid, n.path, e.raw_target FROM edges e
                   JOIN nodes n ON n.id=e.src_id
                 WHERE e.resolved=0 AND e.kind IN ('wikilink','frontmatter_ref')",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        for (rowid, src, raw) in pending {
            if let crate::resolve::Resolution::Resolved(dst) =
                crate::resolve::resolve_target(&src, &raw, &all_paths)
            {
                if let Some(dst_id) = self.node_id(&dst)? {
                    self.conn.execute(
                        "UPDATE edges SET dst_id=?1, resolved=1 WHERE rowid=?2",
                        rusqlite::params![dst_id, rowid],
                    )?;
                }
            }
        }
        Ok(())
    }

    /// For each `index` node (README), add `contains` edges to sibling nodes in
    /// the same directory. Clears existing contains edges first (idempotent).
    pub fn add_contains_edges(&self) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM edges WHERE kind='contains'", [])?;
        let indexes: Vec<(i64, String)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, path FROM nodes WHERE type='index'")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        let all_nodes: Vec<(i64, String)> = {
            let mut stmt = self.conn.prepare("SELECT id, path FROM nodes")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        for (idx_id, idx_path) in indexes {
            let dir = match idx_path.rfind('/') {
                Some(i) => &idx_path[..i],
                None => "",
            };
            for (sib_id, sib_path) in &all_nodes {
                if *sib_id != idx_id && same_dir(sib_path, dir) {
                    self.conn.execute(
                        "INSERT INTO edges(src_id, dst_id, kind, raw_target, anchor, resolved)
                         VALUES (?1, ?2, 'contains', '', NULL, 1)",
                        rusqlite::params![idx_id, sib_id],
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn file_mtime(path: &std::path::Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// True if `path`'s immediate parent directory equals `dir`.
fn same_dir(path: &str, dir: &str) -> bool {
    let parent = match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    };
    parent == dir
}

fn node_type_str(t: crate::model::NodeType) -> &'static str {
    match t {
        crate::model::NodeType::Index => "index",
        crate::model::NodeType::Note => "note",
    }
}

fn scalar_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_memory_and_creates_schema() {
        let idx = Index::open_in_memory().unwrap();
        let tables: i64 = idx
            .conn()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('nodes','node_meta','edges')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 3);
        // FTS5 virtual table exists and is usable
        idx.conn()
            .execute(
                "INSERT INTO fts(rowid, title, body) VALUES (1, 'hi', 'there')",
                [],
            )
            .unwrap();
    }

    #[test]
    fn upsert_note_writes_node_meta_and_unresolved_edges() {
        use crate::parse::parse_note;
        let idx = Index::open_in_memory().unwrap();
        let doc = "---\ntype: spec\nstatus: active\n---\n# T\n\nlink [[other]]\n";
        let n = parse_note("a.md", doc.as_bytes());
        idx.upsert_note(&n, 42).unwrap();

        let (title, ntype): (String, String) = idx
            .conn()
            .query_row("SELECT title, type FROM nodes WHERE path='a.md'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(title, "T");
        assert_eq!(ntype, "note");

        let status: String = idx
            .conn()
            .query_row(
                "SELECT value FROM node_meta m JOIN nodes n ON n.id=m.node_id WHERE n.path='a.md' AND m.key='status'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");

        let (raw, resolved): (String, i64) = idx
            .conn()
            .query_row("SELECT raw_target, resolved FROM edges", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(raw, "other");
        assert_eq!(resolved, 0); // unresolved until the resolve pass
    }

    #[test]
    fn build_resolves_edges_and_adds_contains() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/README.md"), "# Docs\n\nsee [[guide]]\n").unwrap();
        fs::write(
            root.join("docs/guide.md"),
            "# Guide\n\nback to [[README]] and [[missing]]\n",
        )
        .unwrap();

        let idx = Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();

        // [[guide]] from docs/README.md resolves to docs/guide.md
        let resolved_dst: String = idx
            .conn()
            .query_row(
                "SELECT n2.path FROM edges e
                   JOIN nodes n1 ON n1.id=e.src_id
                   JOIN nodes n2 ON n2.id=e.dst_id
                 WHERE n1.path='docs/README.md' AND e.kind='wikilink'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(resolved_dst, "docs/guide.md");

        // [[missing]] stays unresolved
        let unresolved: i64 = idx
            .conn()
            .query_row(
                "SELECT count(*) FROM edges WHERE raw_target='missing' AND resolved=0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(unresolved, 1);

        // contains edge: docs/README.md -> docs/guide.md
        let contains: i64 = idx
            .conn()
            .query_row(
                "SELECT count(*) FROM edges e
                   JOIN nodes n1 ON n1.id=e.src_id JOIN nodes n2 ON n2.id=e.dst_id
                 WHERE e.kind='contains' AND n1.path='docs/README.md' AND n2.path='docs/guide.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(contains, 1);
    }
}
