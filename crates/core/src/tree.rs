use crate::index::Index;
use serde::Serialize;

/// A flat, sorted list of vault nodes carrying enough structure for the webview
/// to render a collapsible tree (group by `dir`). Flat is simpler than nested
/// and the frontend groups it; v1 keeps the engine boring.
#[derive(Debug, Clone, Serialize)]
pub struct TreeEntry {
    pub path: String,
    pub dir: String,
    pub name: String,
    pub title: String,
    pub is_index: bool,
}

pub fn vault_tree(idx: &Index) -> anyhow::Result<Vec<TreeEntry>> {
    let mut stmt = idx
        .conn()
        .prepare("SELECT path, title, type FROM nodes ORDER BY path")?;
    let rows = stmt.query_map([], |r| {
        let path: String = r.get(0)?;
        let title: String = r.get(1)?;
        let type_str: String = r.get(2)?;
        let (dir, name) = match path.rfind('/') {
            Some(i) => (path[..i].to_string(), path[i + 1..].to_string()),
            None => (String::new(), path.clone()),
        };
        Ok(TreeEntry {
            is_index: type_str == "index",
            path,
            dir,
            name,
            title,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn tree_lists_all_nodes_sorted_with_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("top.md"), "# Top").unwrap();
        fs::write(root.join("docs/README.md"), "# Docs").unwrap();
        fs::write(root.join("docs/a.md"), "# A").unwrap();
        let idx = crate::index::Index::open_in_memory().unwrap();
        idx.build(root, ".aiignore").unwrap();

        let entries = vault_tree(&idx).unwrap();
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"top.md"));
        assert!(paths.contains(&"docs/README.md"));
        assert!(paths.contains(&"docs/a.md"));
        let docs_readme = entries.iter().find(|e| e.path == "docs/README.md").unwrap();
        assert_eq!(docs_readme.dir, "docs");
        assert_eq!(docs_readme.name, "README.md");
        assert!(docs_readme.is_index);
    }
}
