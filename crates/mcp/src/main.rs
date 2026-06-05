//! Lattice MCP server (stdio). Thin adapter exposing core queries as MCP tools.
//! Tools implemented in Phase A.

fn main() -> anyhow::Result<()> {
    eprintln!("lattice-mcp {} (scaffold)", lattice_core::version());
    Ok(())
}
