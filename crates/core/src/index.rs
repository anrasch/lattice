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
}
