use ignore::WalkBuilder;
use std::path::Path;

/// Walk `root`, returning POSIX-style vault-relative paths of every `.md` file
/// not excluded by `ignore_file` (gitignore syntax). Hidden files are skipped
/// and git's own ignore sources are NOT consulted; only `ignore_file` governs
/// exclusion.
pub fn walk_vault(root: &Path, ignore_file: &str) -> anyhow::Result<Vec<String>> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(true)
        .parents(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .add_custom_ignore_filename(ignore_file);

    let mut out = Vec::new();
    for result in builder.build() {
        let entry = result?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(path);
        out.push(rel.to_string_lossy().replace('\\', "/"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn lists_md_files_relative_and_respects_aiignore() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("a.md"), "x").unwrap();
        fs::write(root.join("docs/b.md"), "x").unwrap();
        fs::write(root.join("docs/skip.md"), "x").unwrap();
        fs::write(root.join("notes.txt"), "x").unwrap(); // non-md ignored
        fs::write(root.join(".aiignore"), "docs/skip.md\n").unwrap();

        let mut got = walk_vault(root, ".aiignore").unwrap();
        got.sort();
        assert_eq!(got, vec!["a.md".to_string(), "docs/b.md".to_string()]);
    }
}
