use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeType {
    Index,
    Note,
}

impl NodeType {
    pub fn from_path(rel_path: &str) -> Self {
        let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
        if name.eq_ignore_ascii_case("README.md") {
            NodeType::Index
        } else {
            NodeType::Note
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Wikilink,
    FrontmatterRef,
    Contains,
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EdgeKind::Wikilink => "wikilink",
            EdgeKind::FrontmatterRef => "frontmatter_ref",
            EdgeKind::Contains => "contains",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "wikilink" => Some(EdgeKind::Wikilink),
            "frontmatter_ref" => Some(EdgeKind::FrontmatterRef),
            "contains" => Some(EdgeKind::Contains),
            _ => None,
        }
    }
}

/// A parsed wiki-style link: `[[target#anchor|display]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub target: String,
    pub anchor: Option<String>,
    pub display: Option<String>,
}

/// One `.md` file parsed into graph-ready data (no DB ids yet).
#[derive(Debug, Clone)]
pub struct ParsedNote {
    pub rel_path: String,
    pub title: String,
    pub node_type: NodeType,
    pub frontmatter: serde_json::Value,
    pub frontmatter_refs: Vec<String>,
    pub wikilinks: Vec<WikiLink>,
    pub md_link_targets: Vec<String>,
    pub body_text: String,
    pub body_hash: String,
}

/// A node row as returned by queries.
#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub path: String,
    pub title: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
}

/// An edge as returned by queries (source/target by path).
#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub src: String,
    pub dst: Option<String>,
    pub kind: EdgeKind,
    pub raw_target: String,
    pub resolved: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_type_from_path_detects_readme() {
        assert_eq!(NodeType::from_path("docs/README.md"), NodeType::Index);
        assert_eq!(NodeType::from_path("docs/readme.md"), NodeType::Index);
        assert_eq!(NodeType::from_path("docs/note.md"), NodeType::Note);
    }

    #[test]
    fn edge_kind_roundtrips_as_str() {
        for k in [EdgeKind::Wikilink, EdgeKind::FrontmatterRef, EdgeKind::Contains] {
            assert_eq!(EdgeKind::from_str(k.as_str()), Some(k));
        }
    }
}
