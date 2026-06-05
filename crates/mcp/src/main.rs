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
}

#[derive(Deserialize, JsonSchema)]
struct SearchArg {
    text: String,
    limit: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
struct DirArg {
    dir: String,
}

#[derive(Deserialize, JsonSchema)]
struct ContextArg {
    note: String,
    budget: Option<usize>,
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
        description = "Nodes whose frontmatter matches all key=value filters."
    )]
    async fn vault_query(
        &self,
        Parameters(QueryArg { filters }): Parameters<QueryArg>,
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
        json(&self.vault()?.query(&refs).map_err(internal)?)
    }

    #[tool(
        name = "vault_search",
        description = "Full-text search over title and body."
    )]
    async fn vault_search(
        &self,
        Parameters(SearchArg { text, limit }): Parameters<SearchArg>,
    ) -> Result<String, ErrorData> {
        json(
            &self
                .vault()?
                .search(&text, limit.unwrap_or(20))
                .map_err(internal)?,
        )
    }

    #[tool(
        name = "vault_orphans",
        description = "Nodes with no resolved inbound link."
    )]
    async fn vault_orphans(&self) -> Result<String, ErrorData> {
        json(&self.vault()?.orphans().map_err(internal)?)
    }

    #[tool(
        name = "vault_broken_links",
        description = "Unresolved [[link targets]]."
    )]
    async fn vault_broken_links(&self) -> Result<String, ErrorData> {
        json(&self.vault()?.broken_links().map_err(internal)?)
    }

    #[tool(
        name = "vault_index",
        description = "All nodes under a directory (contains tree)."
    )]
    async fn vault_index(
        &self,
        Parameters(DirArg { dir }): Parameters<DirArg>,
    ) -> Result<String, ErrorData> {
        json(&self.vault()?.index_tree(&dir).map_err(internal)?)
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
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LatticeServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = LatticeServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
