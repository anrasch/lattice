//! Lattice core engine: parse a markdown vault into a graph, maintain a derived
//! SQLite index, and answer graph queries. See the design spec for the model.

pub mod bundle;
pub mod config;
pub mod control;
pub mod edit;
pub mod index;
pub mod links;
pub mod model;
pub mod parse;
pub mod query;
pub mod render;
pub mod resolve;
pub mod tree;
pub mod walk;
pub mod watch;
pub mod wikilink;
pub mod write;

pub use config::Config;
pub use index::Index;
pub use model::{Edge, EdgeKind, Node, NodeType};

use std::path::{Path, PathBuf};

/// High-level handle the adapters (MCP/CLI) use: an index bound to a vault root.
pub struct Vault {
    index: Index,
    root: PathBuf,
    ignore_file: String,
}

impl Vault {
    /// Open (or create) the on-disk index for `root` and build it.
    pub fn open(root: &Path, db_path: &Path, ignore_file: &str) -> anyhow::Result<Self> {
        let index = Index::open(db_path)?;
        index.build(root, ignore_file)?;
        Ok(Vault {
            index,
            root: root.to_path_buf(),
            ignore_file: ignore_file.to_string(),
        })
    }

    /// In-memory variant for tests.
    pub fn open_in_memory(root: &Path) -> anyhow::Result<Self> {
        let index = Index::open_in_memory()?;
        index.build(root, ".aiignore")?;
        Ok(Vault {
            index,
            root: root.to_path_buf(),
            ignore_file: ".aiignore".to_string(),
        })
    }

    /// Cheaply revalidate the index against disk (incremental). Call before a
    /// read on a long-lived index so queries are fresh without a full rebuild.
    pub fn sync(&self) -> anyhow::Result<bool> {
        self.index.sync(&self.root, &self.ignore_file)
    }

    pub fn backlinks(&self, note: &str) -> anyhow::Result<Vec<Edge>> {
        query::backlinks(&self.index, note)
    }
    pub fn links(&self, note: &str) -> anyhow::Result<Vec<Edge>> {
        query::links(&self.index, note)
    }
    pub fn query(
        &self,
        filters: &[(&str, &str)],
        under: Option<&str>,
    ) -> anyhow::Result<Vec<Node>> {
        query::query(&self.index, filters, under)
    }
    pub fn search(
        &self,
        text: &str,
        under: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<Node>> {
        query::search(&self.index, text, under, limit)
    }
    pub fn orphans(&self, under: Option<&str>, limit: usize) -> anyhow::Result<Vec<Node>> {
        query::orphans(&self.index, under, limit)
    }
    pub fn broken_links(&self, under: Option<&str>, limit: usize) -> anyhow::Result<Vec<Edge>> {
        query::broken_links(&self.index, under, limit)
    }
    pub fn index_tree(&self, dir: &str, limit: usize) -> anyhow::Result<Vec<Node>> {
        query::index_tree(&self.index, dir, limit)
    }
    pub fn meta_keys(&self) -> anyhow::Result<Vec<query::MetaKey>> {
        query::meta_keys(&self.index)
    }
    pub fn dir_summary(&self) -> anyhow::Result<Vec<query::DirCount>> {
        query::dir_summary(&self.index)
    }
    pub fn changed_since(&self, since: &str, limit: usize) -> anyhow::Result<Vec<Node>> {
        query::changed_since(&self.index, since, limit)
    }
    pub fn superseded(&self, limit: usize) -> anyhow::Result<Vec<Edge>> {
        query::superseded(&self.index, limit)
    }

    /// Rename/move a note, repairing inbound links. Dry-run unless `apply`.
    pub fn rename(
        &mut self,
        from: &str,
        to: &str,
        apply: bool,
    ) -> anyhow::Result<write::RenamePlan> {
        let mut plan = write::plan_rename(&self.index, &self.root, from, to)?;
        if apply {
            write::apply_rename(&self.index, &self.root, &plan)?;
            plan.applied = true;
        }
        Ok(plan)
    }

    /// Patch a note's frontmatter. Dry-run unless `apply`.
    pub fn patch_frontmatter(
        &mut self,
        note: &str,
        set: &[(String, String)],
        add: &[(String, Vec<String>)],
        unset: &[String],
        apply: bool,
    ) -> anyhow::Result<write::PatchPlan> {
        let mut plan = write::plan_patch(&self.index, &self.root, note, set, add, unset)?;
        if apply {
            write::apply_patch(&self.index, &self.root, &plan)?;
            plan.applied = true;
        }
        Ok(plan)
    }
    pub fn context_bundle(&self, note: &str, budget: usize) -> anyhow::Result<bundle::Bundle> {
        bundle::context_bundle(&self.index, &self.root, note, budget)
    }

    /// Rendered (sanitized) HTML for a note.
    pub fn render(&self, note: &str) -> anyhow::Result<String> {
        let raw = edit::read_raw(&self.root, note)?;
        Ok(render::render_html(&raw.content))
    }

    /// Raw note content + hash for the editor.
    pub fn read_raw(&self, note: &str) -> anyhow::Result<edit::RawNote> {
        edit::read_raw(&self.root, note)
    }

    /// Save edited content (hash-guarded). On success, re-index the file so
    /// queries reflect the edit immediately.
    pub fn save(
        &mut self,
        note: &str,
        content: &str,
        expected_hash: &str,
    ) -> anyhow::Result<edit::WriteOutcome> {
        let outcome = edit::write_note(&self.root, note, content, expected_hash)?;
        if matches!(outcome, edit::WriteOutcome::Written { .. }) {
            self.index.reindex_path(&self.root, note)?;
        }
        Ok(outcome)
    }

    /// Sidebar tree entries.
    pub fn tree(&self) -> anyhow::Result<Vec<tree::TreeEntry>> {
        tree::vault_tree(&self.index)
    }

    /// Re-index a single changed file (create/modify/delete) in place, keeping a
    /// long-lived index live without a full rebuild. Used by the watcher.
    pub fn reindex(&mut self, rel: &str) -> anyhow::Result<()> {
        self.index.reindex_path(&self.root, rel)
    }

    /// Look up the tree entry for one path (or `None` if not indexed). Lets the
    /// desktop watcher hand the UI an authoritative entry per changed path.
    pub fn tree_entry(&self, rel: &str) -> anyhow::Result<Option<tree::TreeEntry>> {
        tree::tree_entry(&self.index, rel)
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

    #[test]
    fn vault_render_read_save_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("n.md"), "# Hi\n\nbody\n").unwrap();
        let mut vault = Vault::open_in_memory(root).unwrap();

        let html = vault.render("n.md").unwrap();
        assert!(html.contains("<h1>"));

        let raw = vault.read_raw("n.md").unwrap();
        assert!(raw.content.contains("# Hi"));

        let out = vault.save("n.md", "# Hi\n\nedited\n", &raw.hash).unwrap();
        assert!(matches!(out, edit::WriteOutcome::Written { .. }));
        // index reflects the edit after save
        assert!(vault
            .search("edited", None, 5)
            .unwrap()
            .iter()
            .any(|n| n.path == "n.md"));

        assert!(vault.tree().unwrap().iter().any(|e| e.path == "n.md"));
    }

    #[test]
    fn vault_rename_dry_run_then_apply() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.md"), "# A\n\n[[b]]\n").unwrap();
        std::fs::write(root.join("b.md"), "# B\n").unwrap();
        let mut vault = Vault::open_in_memory(root).unwrap();

        let plan = vault.rename("b.md", "c.md", false).unwrap();
        assert_eq!(plan.links_rewritten, 1);
        assert!(!plan.applied);
        assert!(root.join("b.md").exists());

        let plan = vault.rename("b.md", "c.md", true).unwrap();
        assert!(plan.applied);
        assert!(root.join("c.md").exists());
        assert!(!root.join("b.md").exists());
        assert!(vault
            .backlinks("c.md")
            .unwrap()
            .iter()
            .any(|e| e.src == "a.md"));
    }

    #[test]
    fn vault_reindex_picks_up_a_new_file_without_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.md"), "# A\n").unwrap();
        let mut vault = Vault::open_in_memory(root).unwrap();
        assert!(vault.search("beta", None, 5).unwrap().is_empty());

        // file appears after the index was built; reindex just that path
        std::fs::write(root.join("b.md"), "# B\n\nbeta content\n").unwrap();
        vault.reindex("b.md").unwrap();
        assert!(vault
            .search("beta", None, 5)
            .unwrap()
            .iter()
            .any(|n| n.path == "b.md"));
    }

    #[test]
    fn tree_entry_reflects_reindex_add_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.md"), "# A\n").unwrap();
        let mut vault = Vault::open_in_memory(root).unwrap();
        assert!(vault.tree_entry("b.md").unwrap().is_none());

        std::fs::write(root.join("b.md"), "# B\n").unwrap();
        vault.reindex("b.md").unwrap();
        assert_eq!(vault.tree_entry("b.md").unwrap().unwrap().title, "B");

        std::fs::remove_file(root.join("b.md")).unwrap();
        vault.reindex("b.md").unwrap();
        assert!(vault.tree_entry("b.md").unwrap().is_none());
    }

    #[test]
    fn vault_patch_dry_run_then_apply() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.md"), "---\nstatus: active\n---\n# A\n").unwrap();
        let mut vault = Vault::open_in_memory(root).unwrap();

        let set = vec![("status".to_string(), "shipped".to_string())];
        let plan = vault
            .patch_frontmatter("a.md", &set, &[], &[], false)
            .unwrap();
        assert!(!plan.applied);
        assert!(std::fs::read_to_string(root.join("a.md"))
            .unwrap()
            .contains("active"));

        vault
            .patch_frontmatter("a.md", &set, &[], &[], true)
            .unwrap();
        assert!(std::fs::read_to_string(root.join("a.md"))
            .unwrap()
            .contains("shipped"));
    }
}
