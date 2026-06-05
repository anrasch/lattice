use crate::model::WikiLink;
use once_cell::sync::Lazy;
use regex::Regex;

static WIKILINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[\[([^\]\n]+)\]\]").unwrap());

/// Extract `[[target#anchor|display]]` links from text.
///
/// Display text never affects the target. A bare `[[#anchor]]` (empty target)
/// is an intra-document link and is skipped.
pub fn parse_wikilinks(text: &str) -> Vec<WikiLink> {
    WIKILINK_RE
        .captures_iter(text)
        .filter_map(|c| {
            let inner = c.get(1).unwrap().as_str();
            let (target_part, display) = match inner.split_once('|') {
                Some((t, d)) => (t.trim(), Some(d.trim().to_string())),
                None => (inner.trim(), None),
            };
            let (target, anchor) = match target_part.split_once('#') {
                Some((t, a)) => (t.trim().to_string(), Some(a.trim().to_string())),
                None => (target_part.to_string(), None),
            };
            if target.is_empty() {
                return None;
            }
            Some(WikiLink {
                target,
                anchor,
                display,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(s: &str) -> WikiLink {
        let v = parse_wikilinks(s);
        assert_eq!(v.len(), 1, "expected exactly one link in {s:?}");
        v.into_iter().next().unwrap()
    }

    #[test]
    fn plain_target() {
        let l = one("see [[foo]] now");
        assert_eq!(l.target, "foo");
        assert_eq!(l.anchor, None);
        assert_eq!(l.display, None);
    }

    #[test]
    fn target_with_anchor_and_alias() {
        let l = one("[[notes/foo#Heading Two|the alias]]");
        assert_eq!(l.target, "notes/foo");
        assert_eq!(l.anchor.as_deref(), Some("Heading Two"));
        assert_eq!(l.display.as_deref(), Some("the alias"));
    }

    #[test]
    fn bare_anchor_is_not_a_link() {
        assert!(parse_wikilinks("jump to [[#section]] here").is_empty());
    }

    #[test]
    fn multiple_links() {
        assert_eq!(parse_wikilinks("[[a]] and [[b|B]]").len(), 2);
    }
}
