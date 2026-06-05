# Lattice

Local-first markdown knowledge vault optimized for AI cowork. Query your notes as a graph over MCP, edit them in a native Rust and Tauri app.

Lattice treats a directory of plain `.md` files as a knowledge graph. Humans read and edit through a native app; AI agents (Claude and any other MCP client) navigate the same vault by query instead of by `grep`. The markdown files stay the source of truth; the graph is a derived, rebuildable index.

## Status

Early development. Phase A (core engine + MCP + CLI) is in progress.

## Why

Obsidian's graph and backlinks are built for a human clicking, not for an agent. Lattice exposes the same graph as a machine-facing query API (backlinks, frontmatter filters, orphans, broken links, and a token-budgeted context bundle) over the Model Context Protocol, and ships its own native reading and editing interface so it does not depend on a third-party app.

## Architecture

One Rust `core` crate, three surfaces:

- **MCP server** (`rmcp`, stdio) — the primary AI surface. Agents call typed tools.
- **CLI** (`clap`) — the same queries for scripts and humans.
- **Native app** (Tauri + Svelte) — read, edit, and a live cowork view.

The vault's `.md` files are the source of truth. A SQLite (`rusqlite`, FTS5) index is derived from them and kept in sync by a file watcher (`notify`).

## Primitives

- **Ignore layer** — `.aiignore` (gitignore syntax) keeps system and generated files out of the graph.
- **Nodes** — every `.md` is a node; a directory's `README.md` is an `index` node (its entry point).
- **Edges** — `[[wikilinks]]` and markdown links, frontmatter references, and directory containment, all queryable.
- **Frontmatter** — YAML frontmatter is typed, queryable structured data.

## Layout

```
crates/core   the engine library (parse, index, query, edit)
crates/mcp    rmcp server binary
crates/cli    clap binary
app/          Tauri app (Rust backend + Svelte webview)   [Phase B]
fixtures/     synthetic vault for tests
```

## Usage

Build the workspace:

```bash
cargo build --release
```

### CLI

Run any query against a vault from the shell (JSON output):

```bash
lattice --root /path/to/vault backlinks docs/guide.md
lattice --root /path/to/vault query type=spec status=active
lattice --root /path/to/vault broken-links
lattice --root /path/to/vault context docs/guide.md --budget 8000
```

### As an MCP server

`lattice-mcp` speaks the Model Context Protocol over stdio. Point it at a vault
with `LATTICE_ROOT` and register it with any MCP client. For Claude Code, add to
`.mcp.json`:

```json
{
  "mcpServers": {
    "lattice": {
      "command": "/path/to/lattice/target/release/lattice-mcp",
      "env": { "LATTICE_ROOT": "/path/to/vault", "LATTICE_DB": "/tmp/lattice.db" }
    }
  }
}
```

Tools exposed: `vault_backlinks`, `vault_links`, `vault_query`, `vault_search`,
`vault_orphans`, `vault_broken_links`, `vault_index`, `vault_context_bundle`.

## License

Apache-2.0. See [LICENSE](./LICENSE).
