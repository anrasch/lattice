//! Lattice CLI. Thin adapter over core; mirrors the MCP tools as subcommands.
//! Commands implemented in Phase A.

fn main() -> anyhow::Result<()> {
    println!("lattice {} (scaffold)", lattice_core::version());
    Ok(())
}
