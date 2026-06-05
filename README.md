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

## License

Apache-2.0. See [LICENSE](./LICENSE).
