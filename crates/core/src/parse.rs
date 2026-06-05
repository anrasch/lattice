use crate::model::{NodeType, ParsedNote};
use crate::wikilink::parse_wikilinks;
use once_cell::sync::Lazy;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use regex::Regex;

static INLINE_CODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`[^`\n]*`").unwrap());

/// Remove fenced code blocks and inline code spans so wiki-link extraction
/// ignores `[[example]]` targets quoted as code (e.g. in a spec). FTS body text
/// keeps the code; only link scanning is filtered.
fn strip_code(md: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    for line in md.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    INLINE_CODE_RE.replace_all(&out, " ").into_owned()
}

/// Frontmatter ref fields recognized by default (overridable via Config later).
const DEFAULT_REF_FIELDS: &[&str] = &["related"];

/// Split a leading `---\n ... \n---\n` YAML frontmatter block.
/// Returns (yaml_str, body_str). No frontmatter -> ("", whole input).
fn split_frontmatter(text: &str) -> (&str, &str) {
    let t = text.strip_prefix('\u{feff}').unwrap_or(text); // tolerate BOM
    if let Some(rest) = t.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            return (&rest[..end], &rest[end + 5..]);
        }
        if let Some(end) = rest.find("\n---") {
            // closing fence at EOF without trailing newline
            return (&rest[..end], &rest[(end + 4).min(rest.len())..]);
        }
    }
    ("", t)
}

fn yaml_to_json(yaml: &str) -> serde_json::Value {
    if yaml.trim().is_empty() {
        return serde_json::Value::Object(Default::default());
    }
    match serde_yaml::from_str::<serde_json::Value>(yaml) {
        Ok(v) if v.is_object() => v,
        _ => serde_json::Value::Object(Default::default()),
    }
}

fn frontmatter_refs(fm: &serde_json::Value, ref_fields: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for key in ref_fields {
        match fm.get(*key) {
            Some(serde_json::Value::String(s)) => out.push(s.clone()),
            Some(serde_json::Value::Array(a)) => {
                for item in a {
                    if let Some(s) = item.as_str() {
                        out.push(s.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn filename_stem(rel_path: &str) -> String {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

pub fn parse_note(rel_path: &str, bytes: &[u8]) -> ParsedNote {
    let text = String::from_utf8_lossy(bytes);
    let body_hash = blake3::hash(bytes).to_hex().to_string();
    let (yaml, body) = split_frontmatter(&text);
    let frontmatter = yaml_to_json(yaml);
    let supersedes = frontmatter_refs(&frontmatter, &["supersedes"]);
    let frontmatter_refs = frontmatter_refs(&frontmatter, DEFAULT_REF_FIELDS);

    // Walk the markdown once for: first H1 title, internal .md links, plain body text.
    let mut first_h1: Option<String> = None;
    let mut in_h1 = false;
    let mut md_link_targets = Vec::new();
    let mut body_text = String::new();
    let parser = Parser::new(body);
    for ev in parser {
        match ev {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => in_h1 = true,
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => in_h1 = false,
            Event::Start(Tag::Link { dest_url, .. }) => {
                let d = dest_url.to_string();
                let is_external = d.contains("://") || d.starts_with("mailto:");
                if !is_external && (d.ends_with(".md") || !d.contains('.')) {
                    md_link_targets.push(d);
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if in_h1 && first_h1.is_none() {
                    first_h1 = Some(t.to_string());
                }
                body_text.push_str(&t);
                body_text.push(' ');
            }
            _ => {}
        }
    }

    let title = first_h1
        .or_else(|| {
            frontmatter
                .get("title")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| filename_stem(rel_path));

    ParsedNote {
        rel_path: rel_path.to_string(),
        title,
        node_type: NodeType::from_path(rel_path),
        frontmatter,
        frontmatter_refs,
        supersedes,
        wikilinks: parse_wikilinks(&strip_code(body)),
        md_link_targets,
        body_text: body_text.trim().to_string(),
        body_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\ntype: spec\nstatus: active\nrelated: [foo, bar]\n---\n# My Title\n\nBody with [[wiki]] and [a link](other.md) and [ext](https://x.com).\n";

    #[test]
    fn extracts_frontmatter_title_and_links() {
        let n = parse_note("docs/note.md", DOC.as_bytes());
        assert_eq!(n.rel_path, "docs/note.md");
        assert_eq!(n.title, "My Title");
        assert_eq!(n.frontmatter["type"], "spec");
        assert_eq!(n.frontmatter_refs, vec!["foo", "bar"]);
        assert_eq!(n.wikilinks.len(), 1);
        assert_eq!(n.wikilinks[0].target, "wiki");
        assert_eq!(n.md_link_targets, vec!["other.md"]); // external http dropped
        assert!(n.body_text.contains("Body with"));
        assert_eq!(n.body_hash.len(), 64); // blake3 hex
    }

    #[test]
    fn title_falls_back_to_filename_when_no_h1_or_frontmatter() {
        let n = parse_note("docs/some-note.md", b"just text, no heading");
        assert_eq!(n.title, "some-note");
    }

    #[test]
    fn readme_is_index_type() {
        let n = parse_note("docs/README.md", b"# Docs");
        assert_eq!(n.node_type, crate::model::NodeType::Index);
    }

    #[test]
    fn wikilinks_in_code_are_ignored() {
        let doc = "Real [[live]] link.\n\nInline `[[fake]]` code.\n\n```\n[[alsofake]]\n```\n";
        let n = parse_note("a.md", doc.as_bytes());
        let targets: Vec<&str> = n.wikilinks.iter().map(|w| w.target.as_str()).collect();
        assert_eq!(targets, vec!["live"]);
    }
}
