//! Lattice core engine.
//!
//! Parses a markdown vault into a graph (nodes, edges, frontmatter), maintains a
//! derived SQLite index, answers graph queries, and writes edits back to disk.
//! The MCP server, CLI, and Tauri app are thin adapters over this crate.
//!
//! See the design spec for the full model. Implementation lands in Phase A.

/// Placeholder so the workspace builds before Phase A lands.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
