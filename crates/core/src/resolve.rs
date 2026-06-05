#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Resolved(String),
    Ambiguous(Vec<String>),
    NotFound,
}

fn with_md(s: &str) -> String {
    if s.ends_with(".md") {
        s.to_string()
    } else {
        format!("{s}.md")
    }
}

fn parent_dir(rel_path: &str) -> &str {
    match rel_path.rfind('/') {
        Some(i) => &rel_path[..i],
        None => "",
    }
}

fn basename(rel_path: &str) -> &str {
    rel_path.rsplit('/').next().unwrap_or(rel_path)
}

/// Resolve a wiki/markdown target (no anchor/alias) to a vault-relative path,
/// per the v1 rule: path-qualified = exact; bare name = same-dir preference,
/// then unique basename, else ambiguous/not-found.
pub fn resolve_target(source: &str, target: &str, all_paths: &[String]) -> Resolution {
    let target = target.trim().trim_start_matches("./");

    if target.contains('/') {
        let want = with_md(target);
        return if all_paths.iter().any(|p| p == &want) {
            Resolution::Resolved(want)
        } else {
            Resolution::NotFound
        };
    }

    let want_name = with_md(target);

    // (a) same-directory sibling
    let dir = parent_dir(source);
    let same_dir = if dir.is_empty() {
        want_name.clone()
    } else {
        format!("{dir}/{want_name}")
    };
    if all_paths.iter().any(|p| p == &same_dir) {
        return Resolution::Resolved(same_dir);
    }

    // (b)/(c) basename match across the vault
    let matches: Vec<String> = all_paths
        .iter()
        .filter(|p| basename(p) == want_name)
        .cloned()
        .collect();
    match matches.len() {
        1 => Resolution::Resolved(matches.into_iter().next().unwrap()),
        0 => Resolution::NotFound,
        _ => Resolution::Ambiguous(matches),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> Vec<String> {
        vec![
            "README.md".into(),
            "docs/README.md".into(),
            "docs/guide.md".into(),
            "docs/sub/guide.md".into(),
            "solo.md".into(),
        ]
    }

    #[test]
    fn same_directory_preference_wins_for_readme() {
        // From inside docs/, [[README]] hits docs/README.md, not root README.
        assert_eq!(
            resolve_target("docs/guide.md", "README", &paths()),
            Resolution::Resolved("docs/README.md".into())
        );
    }

    #[test]
    fn unique_basename_resolves_across_folders() {
        assert_eq!(
            resolve_target("README.md", "solo", &paths()),
            Resolution::Resolved("solo.md".into())
        );
    }

    #[test]
    fn ambiguous_basename_is_reported_with_candidates() {
        let r = resolve_target("README.md", "guide", &paths());
        match r {
            Resolution::Ambiguous(mut c) => {
                c.sort();
                assert_eq!(
                    c,
                    vec!["docs/guide.md".to_string(), "docs/sub/guide.md".to_string()]
                );
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn path_qualified_target_is_exact() {
        assert_eq!(
            resolve_target("solo.md", "docs/guide", &paths()),
            Resolution::Resolved("docs/guide.md".into())
        );
        assert_eq!(
            resolve_target("solo.md", "docs/missing", &paths()),
            Resolution::NotFound
        );
    }
}
