use crate::index::Index;
use crate::query::links;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct BundleEntry {
    pub path: String,
    pub included: bool,
    pub tokens: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub root: String,
    pub budget: usize,
    pub used: usize,
    pub markdown: String,
    pub manifest: Vec<BundleEntry>,
}

fn count_tokens(s: &str) -> usize {
    // cl100k_base is a good-enough heuristic; budgets are soft.
    match tiktoken_rs::cl100k_base() {
        Ok(bpe) => bpe.encode_with_special_tokens(s).len(),
        Err(_) => s.split_whitespace().count(),
    }
}

fn read_note(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_default()
}

/// Assemble `note` + its 1-hop resolved neighbors, deduped, packed to `budget`
/// tokens (root always included). Returns concatenated markdown + a manifest of
/// what was included/dropped. No silent truncation: dropped notes are listed.
pub fn context_bundle(
    idx: &Index,
    root: &Path,
    note: &str,
    budget: usize,
) -> anyhow::Result<Bundle> {
    // Ordered, deduped candidate list: root first, then resolved neighbors.
    let mut order: Vec<String> = vec![note.to_string()];
    for e in links(idx, note)? {
        if let Some(dst) = e.dst {
            if !order.contains(&dst) {
                order.push(dst);
            }
        }
    }

    let mut markdown = String::new();
    let mut manifest = Vec::new();
    let mut used = 0usize;

    for (i, rel) in order.iter().enumerate() {
        let body = read_note(root, rel);
        let tokens = count_tokens(&body);
        let is_root = i == 0;
        let fits = used + tokens <= budget;
        let include = is_root || fits; // root always in, even if over budget
        if include {
            if !markdown.is_empty() {
                markdown.push_str("\n\n---\n\n");
            }
            markdown.push_str(&body);
            used += tokens;
        }
        manifest.push(BundleEntry {
            path: rel.clone(),
            included: include,
            tokens,
        });
    }

    Ok(Bundle {
        root: note.to_string(),
        budget,
        used,
        markdown,
        manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn built() -> (tempfile::TempDir, crate::index::Index, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        fs::write(root.join("hub.md"), "# Hub\n\nlinks [[leaf]]\n").unwrap();
        fs::write(root.join("leaf.md"), "# Leaf\n\nleaf body content\n").unwrap();
        let idx = crate::index::Index::open_in_memory().unwrap();
        idx.build(&root, ".aiignore").unwrap();
        (dir, idx, root)
    }

    #[test]
    fn bundle_includes_root_and_neighbor_within_budget() {
        let (_d, idx, root) = built();
        let b = context_bundle(&idx, &root, "hub.md", 5000).unwrap();
        assert!(b.markdown.contains("# Hub"));
        assert!(b.markdown.contains("# Leaf"));
        assert!(b.manifest.iter().any(|m| m.path == "hub.md" && m.included));
        assert!(b.manifest.iter().any(|m| m.path == "leaf.md" && m.included));
    }

    #[test]
    fn tiny_budget_drops_neighbors_but_keeps_root() {
        let (_d, idx, root) = built();
        let b = context_bundle(&idx, &root, "hub.md", 1).unwrap();
        assert!(b.manifest.iter().any(|m| m.path == "hub.md" && m.included));
        assert!(b
            .manifest
            .iter()
            .any(|m| m.path == "leaf.md" && !m.included));
    }
}
