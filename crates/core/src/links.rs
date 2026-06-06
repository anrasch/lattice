/// Directory portion of a vault-relative path ("" for a root file).
fn dir_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// POSIX relative path from `source`'s directory to the `target` file.
/// Same dir -> bare filename; otherwise `../` segments + tail.
pub fn relative_path(source: &str, target: &str) -> String {
    let src_dir: Vec<&str> = {
        let d = dir_of(source);
        if d.is_empty() {
            vec![]
        } else {
            d.split('/').collect()
        }
    };
    let tgt: Vec<&str> = target.split('/').collect();
    let tgt_dirs = &tgt[..tgt.len() - 1];

    let mut i = 0;
    while i < src_dir.len() && i < tgt_dirs.len() && src_dir[i] == tgt_dirs[i] {
        i += 1;
    }
    let ups = src_dir.len() - i;
    let mut parts: Vec<String> = std::iter::repeat("..".to_string()).take(ups).collect();
    for seg in &tgt[i..] {
        parts.push((*seg).to_string());
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_directory_is_bare_filename() {
        assert_eq!(relative_path("docs/a.md", "docs/b.md"), "b.md");
    }

    #[test]
    fn up_and_over() {
        assert_eq!(
            relative_path("apps/sweight/README.md", "docs/infra/run.md"),
            "../../docs/infra/run.md"
        );
    }

    #[test]
    fn root_source_into_subdir() {
        assert_eq!(relative_path("top.md", "docs/x.md"), "docs/x.md");
    }
}
