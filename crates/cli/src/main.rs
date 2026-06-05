//! Lattice CLI. Thin adapter over lattice-core; prints JSON by default.

use anyhow::Result;
use clap::{Parser, Subcommand};
use lattice_core::Vault;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lattice", about = "Query a markdown vault as a graph")]
struct Cli {
    /// Vault root (defaults to current directory).
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Ignore file name.
    #[arg(long, default_value = ".aiignore")]
    ignore_file: String,
    /// Index db path.
    #[arg(long, default_value = "lattice.db")]
    db: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Nodes linking TO a note.
    Backlinks { note: String },
    /// Nodes a note links OUT to.
    Links { note: String },
    /// Nodes with no resolved inbound link.
    Orphans {
        #[arg(long)]
        dir: Option<String>,
        #[arg(long, default_value_t = 1000)]
        limit: usize,
    },
    /// Unresolved `[[targets]]`.
    BrokenLinks {
        #[arg(long)]
        dir: Option<String>,
        #[arg(long, default_value_t = 1000)]
        limit: usize,
    },
    /// Nodes whose frontmatter matches all key=value pairs.
    Query { filters: Vec<String> },
    /// Full-text search over title + body.
    Search {
        text: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// All nodes under a directory (the contains tree).
    Index { dir: String },
    /// Token-budgeted context bundle for a note.
    Context {
        note: String,
        #[arg(long, default_value_t = 8000)]
        budget: usize,
    },
}

fn print_json<T: serde::Serialize>(v: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let vault = Vault::open(&cli.root, &cli.db, &cli.ignore_file)?;
    match cli.cmd {
        Cmd::Backlinks { note } => print_json(&vault.backlinks(&note)?),
        Cmd::Links { note } => print_json(&vault.links(&note)?),
        Cmd::Orphans { dir, limit } => print_json(&vault.orphans(dir.as_deref(), limit)?),
        Cmd::BrokenLinks { dir, limit } => print_json(&vault.broken_links(dir.as_deref(), limit)?),
        Cmd::Query { filters } => {
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
            print_json(&vault.query(&refs)?)
        }
        Cmd::Search { text, limit } => print_json(&vault.search(&text, limit)?),
        Cmd::Index { dir } => print_json(&vault.index_tree(&dir)?),
        Cmd::Context { note, budget } => print_json(&vault.context_bundle(&note, budget)?),
    }
}
