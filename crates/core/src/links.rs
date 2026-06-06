use crate::resolve::{resolve_target, Resolution};
use once_cell::sync::Lazy;
use regex::Regex;

static WIKILINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\[([^\]\n]+)\]\]").unwrap());
static MDLINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\]\(([^)\s]+)\)").unwrap());

/// Directory portion of a vault-relative path ("" for a root file).
fn dir_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

fn strip_md(s: &str) -> &str {
    s.strip_suffix(".md").unwrap_or(s)
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
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_string(), ups).collect();
    for seg in &tgt[i..] {
        parts.push((*seg).to_string());
    }
    parts.join("/")
}

/// The new wiki/frontmatter target string to use from `source`, given the link
/// previously used `old_raw`. Preserves bare-vs-qualified style; falls back to
/// path-qualified when a bare basename would be ambiguous against `post` (the
/// path set with the rename already applied).
fn new_target_str(source: &str, old_raw: &str, new_path: &str, post: &[String]) -> String {
    let new_noext = strip_md(new_path).to_string();
    if old_raw.contains('/') {
        return new_noext; // was path-qualified -> stay vault-root-relative
    }
    let base = strip_md(new_path.rsplit('/').next().unwrap_or(new_path)).to_string();
    match resolve_target(source, &base, post) {
        Resolution::Resolved(p) if strip_md(&p) == new_noext => base,
        _ => new_noext,
    }
}

/// Rewrite every wiki-link and markdown link in `content` that resolves to
/// `old_path` so it points at `new_path`, preserving anchor + alias (wiki) and
/// recomputing relative paths (markdown). `all_paths` is the PRE-rename path set
/// (so the old link still resolves); naming of the new target is judged against
/// the post-rename set, derived internally. Returns (new_content, count).
pub fn rewrite_inbound(
    content: &str,
    source: &str,
    old_path: &str,
    new_path: &str,
    all_paths: &[String],
) -> (String, usize) {
    let post: Vec<String> = all_paths
        .iter()
        .map(|p| {
            if p == old_path {
                new_path.to_string()
            } else {
                p.clone()
            }
        })
        .collect();
    let mut count = 0;

    // Wiki-links.
    let out = WIKILINK_RE
        .replace_all(content, |caps: &regex::Captures| {
            let inner = &caps[1];
            let (target_part, alias) = match inner.split_once('|') {
                Some((t, a)) => (t, Some(a)),
                None => (inner, None),
            };
            let (target, anchor) = match target_part.split_once('#') {
                Some((t, a)) => (t, Some(a)),
                None => (target_part, None),
            };
            let hit = matches!(
                resolve_target(source, target.trim(), all_paths),
                Resolution::Resolved(ref p) if p == old_path
            );
            if !hit {
                return caps[0].to_string();
            }
            count += 1;
            let mut s = new_target_str(source, target.trim(), new_path, &post);
            if let Some(a) = anchor {
                s.push('#');
                s.push_str(a);
            }
            if let Some(a) = alias {
                s.push('|');
                s.push_str(a);
            }
            format!("[[{s}]]")
        })
        .into_owned();

    // Markdown links: ](url)
    let out = MDLINK_RE
        .replace_all(&out, |caps: &regex::Captures| {
            let url = &caps[1];
            if url.contains("://") || url.starts_with('#') || url.starts_with("mailto:") {
                return caps[0].to_string();
            }
            let target = strip_md(url.trim_start_matches("./"));
            let hit = matches!(
                resolve_target(source, target, all_paths),
                Resolution::Resolved(ref p) if p == old_path
            );
            if !hit {
                return caps[0].to_string();
            }
            count += 1;
            format!("]({})", relative_path(source, new_path))
        })
        .into_owned();

    (out, count)
}

/// Recompute the moved file's own markdown relative links so they still resolve
/// from `new_source`. `old_source` resolves each link against the (pre-move)
/// vault. Returns (new_content, count).
pub fn rebase_relative(
    content: &str,
    old_source: &str,
    new_source: &str,
    all_paths: &[String],
) -> (String, usize) {
    let mut count = 0;
    let out = MDLINK_RE
        .replace_all(content, |caps: &regex::Captures| {
            let url = &caps[1];
            if url.contains("://") || url.starts_with('#') || url.starts_with("mailto:") {
                return caps[0].to_string();
            }
            let target = strip_md(url.trim_start_matches("./"));
            match resolve_target(old_source, target, all_paths) {
                Resolution::Resolved(dst) => {
                    count += 1;
                    format!("]({})", relative_path(new_source, &dst))
                }
                _ => caps[0].to_string(),
            }
        })
        .into_owned();
    (out, count)
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

    fn paths() -> Vec<String> {
        vec![
            "docs/old.md".into(),
            "docs/other.md".into(),
            "top.md".into(),
        ]
    }

    #[test]
    fn rewrites_bare_wikilink_keeping_alias_and_anchor() {
        let (out, n) = rewrite_inbound(
            "see [[old]] and [[old#Heading|the alias]] here",
            "docs/note.md",
            "docs/old.md",
            "docs/new.md",
            &paths(),
        );
        assert_eq!(n, 2);
        assert_eq!(out, "see [[new]] and [[new#Heading|the alias]] here");
    }

    #[test]
    fn rewrites_path_qualified_wikilink() {
        let (out, n) = rewrite_inbound(
            "[[docs/old]]",
            "top.md",
            "docs/old.md",
            "archive/old.md",
            &paths(),
        );
        assert_eq!(n, 1);
        assert_eq!(out, "[[archive/old]]");
    }

    #[test]
    fn leaves_unrelated_wikilinks_untouched() {
        let (out, n) = rewrite_inbound(
            "[[other]]",
            "docs/note.md",
            "docs/old.md",
            "docs/new.md",
            &paths(),
        );
        assert_eq!(n, 0);
        assert_eq!(out, "[[other]]");
    }

    #[test]
    fn rewrites_markdown_relative_link() {
        let p = vec!["docs/old.md".to_string(), "docs/sub/note.md".to_string()];
        let (out, n) = rewrite_inbound(
            "see [the doc](../old.md) please",
            "docs/sub/note.md",
            "docs/old.md",
            "docs/new.md",
            &p,
        );
        assert_eq!(n, 1);
        assert_eq!(out, "see [the doc](../new.md) please");
    }

    #[test]
    fn leaves_external_markdown_links_untouched() {
        let p = vec!["docs/old.md".to_string()];
        let (out, n) = rewrite_inbound(
            "[site](https://x.com/old.md)",
            "docs/note.md",
            "docs/old.md",
            "docs/new.md",
            &p,
        );
        assert_eq!(n, 0);
        assert_eq!(out, "[site](https://x.com/old.md)");
    }

    #[test]
    fn rebase_recomputes_relative_links_from_new_location() {
        let p = vec!["docs/guide.md".to_string(), "docs/sub/old.md".to_string()];
        let (out, n) = rebase_relative(
            "see [g](../guide.md)",
            "docs/sub/old.md",
            "archive/old.md",
            &p,
        );
        assert_eq!(n, 1);
        assert_eq!(out, "see [g](../docs/guide.md)");
    }
}
