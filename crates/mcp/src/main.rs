//! Lattice MCP server (stdio). Thin adapter exposing lattice-core queries as
//! Model Context Protocol tools. Each tool returns the same JSON the CLI prints.
//!
//! Config via environment: `LATTICE_ROOT` (vault root, default `.`) and
//! `LATTICE_DB` (index path, default `lattice.db`).

use lattice_core::Vault;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router,
    transport::io::stdio,
    ErrorData, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone)]
pub struct LatticeServer {
    root: PathBuf,
    db: PathBuf,
    ignore_file: String,
    tool_router: ToolRouter<Self>,
}

impl LatticeServer {
    fn new() -> Self {
        let root = std::env::var("LATTICE_ROOT").unwrap_or_else(|_| ".".into());
        let db = std::env::var("LATTICE_DB").unwrap_or_else(|_| "lattice.db".into());
        Self {
            root: PathBuf::from(root),
            db: PathBuf::from(db),
            ignore_file: ".aiignore".to_string(),
            tool_router: Self::tool_router(),
        }
    }

    /// Open and build the vault fresh per call. Stateless by necessity: a
    /// rusqlite `Connection` is not `Sync`, so the handler can't hold a `Vault`.
    /// Acceptable for v1; a long-lived shared index is a later optimization.
    fn vault(&self) -> Result<Vault, ErrorData> {
        Vault::open(&self.root, &self.db, &self.ignore_file).map_err(internal)
    }
}

fn internal(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(e.to_string(), None)
}

fn json<T: serde::Serialize>(v: &T) -> Result<String, ErrorData> {
    serde_json::to_string(v).map_err(internal)
}

#[derive(Deserialize, JsonSchema)]
struct NoteArg {
    note: String,
}

#[derive(Deserialize, JsonSchema)]
struct QueryArg {
    /// Frontmatter filters as `key=value` strings; all must match.
    filters: Vec<String>,
    /// Optional directory prefix to scope to (e.g. "docs").
    dir: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct SearchArg {
    text: String,
    /// Optional directory prefix to scope to (e.g. "docs").
    dir: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct DirArg {
    dir: String,
    /// Max results (default 200).
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct SinceArg {
    /// ISO date (e.g. "2026-06-01"); returns notes updated on/after it.
    since: String,
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct LimitArg {
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct ScopeArg {
    /// Optional directory prefix to scope results to (e.g. "docs").
    dir: Option<String>,
    /// Maximum results to return (default 200).
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct ContextArg {
    note: String,
    budget: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct RenameArg {
    from: String,
    to: String,
    /// Write the change. Omit/false to preview (returns a diff plan).
    apply: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
struct PatchArg {
    note: String,
    /// Replace key values: ["status=shipped", "updated=2026-06-06"].
    #[serde(default)]
    set: Vec<String>,
    /// Append to list fields: ["supersedes=docs/old.md"].
    #[serde(default)]
    add: Vec<String>,
    /// Remove keys: ["draft"].
    #[serde(default)]
    unset: Vec<String>,
    /// Write the change. Omit/false to preview.
    apply: Option<bool>,
}

#[tool_router(router = tool_router)]
impl LatticeServer {
    #[tool(name = "vault_backlinks", description = "Nodes linking to a note.")]
    async fn vault_backlinks(
        &self,
        Parameters(NoteArg { note }): Parameters<NoteArg>,
    ) -> Result<String, ErrorData> {
        json(&self.vault()?.backlinks(&note).map_err(internal)?)
    }

    #[tool(name = "vault_links", description = "Nodes a note links out to.")]
    async fn vault_links(
        &self,
        Parameters(NoteArg { note }): Parameters<NoteArg>,
    ) -> Result<String, ErrorData> {
        json(&self.vault()?.links(&note).map_err(internal)?)
    }

    #[tool(
        name = "vault_query",
        description = "Nodes whose frontmatter matches all key=value filters. Call vault_keys first to discover valid keys/values. Note: `type` is the frontmatter field (e.g. spec/reference), not the structural index/note kind."
    )]
    async fn vault_query(
        &self,
        Parameters(QueryArg { filters, dir }): Parameters<QueryArg>,
    ) -> Result<String, ErrorData> {
        let pairs: Vec<(String, String)> = filters
            .iter()
            .filter_map(|f| {
                f.split_once('=')
                    .map(|(k, v)| (k.to_string(), v.to_string()))
            })
            .collect();
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        json(
            &self
                .vault()?
                .query(&refs, dir.as_deref())
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_search",
        description = "Full-text search over title and body."
    )]
    async fn vault_search(
        &self,
        Parameters(SearchArg { text, dir, limit }): Parameters<SearchArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .search(&text, dir.as_deref(), limit.unwrap_or(20))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_orphans",
        description = "Nodes with no resolved inbound link. Optional dir scope + limit (default 200)."
    )]
    async fn vault_orphans(
        &self,
        Parameters(ScopeArg { dir, limit }): Parameters<ScopeArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .orphans(dir.as_deref(), limit.unwrap_or(200))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_broken_links",
        description = "Unresolved [[link targets]]. Optional dir scope + limit (default 200)."
    )]
    async fn vault_broken_links(
        &self,
        Parameters(ScopeArg { dir, limit }): Parameters<ScopeArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .broken_links(dir.as_deref(), limit.unwrap_or(200))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_index",
        description = "Notes under a directory (contains tree), capped at limit (default 200). For the whole-vault shape use vault_dirs instead of \"/\"."
    )]
    async fn vault_index(
        &self,
        Parameters(DirArg { dir, limit }): Parameters<DirArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .index_tree(&dir, limit.unwrap_or(200))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_dirs",
        description = "Budget-friendly map of the vault: every directory with its note count."
    )]
    async fn vault_dirs(&self) -> Result<String, ErrorData> {
        json(&self.vault()?.dir_summary().map_err(internal)?)
    }

    #[tool(
        name = "vault_keys",
        description = "Enumerate frontmatter keys with their values + counts, to discover vault_query filters."
    )]
    async fn vault_keys(&self) -> Result<String, ErrorData> {
        json(&self.vault()?.meta_keys().map_err(internal)?)
    }

    #[tool(
        name = "vault_changed_since",
        description = "Notes whose updated/date frontmatter is on/after an ISO date (newest first). Re-ground a session on just the deltas."
    )]
    async fn vault_changed_since(
        &self,
        Parameters(SinceArg { since, limit }): Parameters<SinceArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .changed_since(&since, limit.unwrap_or(100))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_superseded",
        description = "Supersession edges (src supersedes dst). A dst here is an overruled decision — read the superseding note instead."
    )]
    async fn vault_superseded(
        &self,
        Parameters(LimitArg { limit }): Parameters<LimitArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .superseded(limit.unwrap_or(200))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_context_bundle",
        description = "Token-budgeted context bundle for a note (markdown + manifest)."
    )]
    async fn vault_context_bundle(
        &self,
        Parameters(ContextArg { note, budget }): Parameters<ContextArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .context_bundle(&note, budget.unwrap_or(8000))
                .map_err(internal)?,
        )
    }
    #[tool(
        name = "vault_rename",
        description = "Move a note and repair every inbound link (wiki/markdown/frontmatter). Dry-run by default; pass apply:true to write. Never touches git."
    )]
    async fn vault_rename(
        &self,
        Parameters(RenameArg { from, to, apply }): Parameters<RenameArg>,
    ) -> Result<String, ErrorData> {
        let mut v = self.vault()?;
        json(&v
            .rename(&from, &to, apply.unwrap_or(false))
            .map_err(internal)?)
    }

    #[tool(
        name = "vault_patch_frontmatter",
        description = "Edit a note's frontmatter (set key=value, add to list fields, unset keys), preserving untouched lines. Dry-run by default; apply:true to write."
    )]
    async fn vault_patch_frontmatter(
        &self,
        Parameters(PatchArg { note, set, add, unset, apply }): Parameters<PatchArg>,
    ) -> Result<String, ErrorData> {
        let set: Vec<(String, String)> = set
            .iter()
            .filter_map(|s| s.split_once('=').map(|(k, v)| (k.to_string(), v.to_string())))
            .collect();
        let add: Vec<(String, Vec<String>)> = add
            .iter()
            .filter_map(|s| s.split_once('=').map(|(k, v)| (k.to_string(), vec![v.to_string()])))
            .collect();
        let mut v = self.vault()?;
        json(&v
            .patch_frontmatter(&note, &set, &add, &unset, apply.unwrap_or(false))
            .map_err(internal)?)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LatticeServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = LatticeServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
