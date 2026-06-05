//! Lattice core engine: parse a markdown vault into a graph, maintain a derived
//! SQLite index, and answer graph queries. See the design spec for the model.

pub mod config;
pub mod model;
pub mod wikilink;
pub mod parse;
pub mod walk;
pub mod resolve;
pub mod index;
pub mod query;
pub mod bundle;
pub mod watch;

/// Crate version, used by the scaffold binaries until they gain real commands.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
