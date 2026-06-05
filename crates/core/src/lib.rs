//! Lattice core engine: parse a markdown vault into a graph, maintain a derived
//! SQLite index, and answer graph queries. See the design spec for the model.

pub mod bundle;
pub mod config;
pub mod edit;
pub mod index;
pub mod model;
pub mod parse;
pub mod query;
pub mod render;
pub mod resolve;
pub mod walk;
pub mod watch;
pub mod wikilink;

pub use config::Config;
pub use index::Index;
pub use model::{Edge, EdgeKind, Node, NodeType};

use std::path::{Path, PathBuf};

/// High-level handle the adapters (MCP/CLI) use: an index bound to a vault root.
pub struct Vault {
    index: Index,
    root: PathBuf,
}

impl Vault {
    /// Open (or create) the on-disk index for `root` and build it.
    pub fn open(root: &Path, db_path: &Path, ignore_file: &str) -> anyhow::Result<Self> {
        let index = Index::open(db_path)?;
        index.build(root, ignore_file)?;
        Ok(Vault {
            index,
            root: root.to_path_buf(),
        })
    }

    /// In-memory variant for tests.
    pub fn open_in_memory(root: &Path) -> anyhow::Result<Self> {
        let index = Index::open_in_memory()?;
        index.build(root, ".aiignore")?;
        Ok(Vault {
            index,
            root: root.to_path_buf(),
        })
    }

    pub fn backlinks(&self, note: &str) -> anyhow::Result<Vec<Edge>> {
        query::backlinks(&self.index, note)
    }
    pub fn links(&self, note: &str) -> anyhow::Result<Vec<Edge>> {
        query::links(&self.index, note)
    }
    pub fn query(&self, filters: &[(&str, &str)]) -> anyhow::Result<Vec<Node>> {
        query::query(&self.index, filters)
    }
    pub fn search(&self, text: &str, limit: usize) -> anyhow::Result<Vec<Node>> {
        query::search(&self.index, text, limit)
    }
    pub fn orphans(&self) -> anyhow::Result<Vec<Node>> {
        query::orphans(&self.index)
    }
    pub fn broken_links(&self) -> anyhow::Result<Vec<Edge>> {
        query::broken_links(&self.index)
    }
    pub fn index_tree(&self, dir: &str) -> anyhow::Result<Vec<Node>> {
        query::index_tree(&self.index, dir)
    }
    pub fn context_bundle(&self, note: &str, budget: usize) -> anyhow::Result<bundle::Bundle> {
        bundle::context_bundle(&self.index, &self.root, note, budget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn vault_opens_builds_and_answers_queries() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.md"), "# A\n\n[[b]]\n").unwrap();
        fs::write(root.join("b.md"), "# B\n").unwrap();

        let vault = Vault::open_in_memory(root).unwrap();
        let bl = vault.backlinks("b.md").unwrap();
        assert!(bl.iter().any(|e| e.src == "a.md"));
        // results serialize to JSON for the adapters
        let json = serde_json::to_string(&bl).unwrap();
        assert!(json.contains("\"src\":\"a.md\""));
    }
}
