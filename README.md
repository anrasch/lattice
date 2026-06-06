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

**Read tools:** `vault_backlinks`, `vault_links`, `vault_query`, `vault_search`,
`vault_orphans`, `vault_broken_links`, `vault_index`, `vault_dirs`, `vault_keys`,
`vault_changed_since`, `vault_superseded`, `vault_context_bundle`.

**Write tools (structure only):** `vault_rename` (move a note + repair every
inbound link) and `vault_patch_frontmatter` (set/add/unset keys, surgical YAML).
Both are **dry-run by default** — they return a diff plan; pass `apply: true` to
write. Lattice only ever edits working-tree files; it never stages or commits to
git, and it never writes body prose (use your editor for that).

```bash
# preview a rename (nothing written); add --apply to do it
lattice --root /path/to/vault rename docs/old.md docs/new.md
lattice --root /path/to/vault patch docs/spec.md --set status=shipped --apply
```

### Make your agent actually use it

Registering the server makes the tools *available*, but agents default to
`grep`/read out of habit. Add a directive to your project's agent instructions
(`CLAUDE.md`, `AGENTS.md`, or equivalent) so it reaches for Lattice by default.
A starting point you can paste and trim:

```markdown
## Knowledge base — use Lattice, not grep

This repo is indexed by the `lattice` MCP server (`vault_*` tools). Prefer them
over raw grep/read — they return structure and relationships, not file dumps:

- "Where is X documented / what links here?" → `vault_search`, `vault_backlinks`,
  then `vault_context_bundle` to load a token-budgeted bundle before a task.
- Shape & discovery → `vault_dirs`, `vault_index`, `vault_keys` → `vault_query`.
- Hygiene → `vault_broken_links`, `vault_orphans`, `vault_changed_since`,
  `vault_superseded` (don't cite a reversed decision).
- Structural edits → `vault_rename` (move a note + fix every backlink) and
  `vault_patch_frontmatter` (set/add/unset keys). Both are dry-run by default:
  preview the diff, then re-call with `apply:true`. They never touch git.

Lattice only sees the indexed vault and only edits structure (location/
frontmatter/links); body prose stays on your normal edit tools.
```

## License

Apache-2.0. See [LICENSE](./LICENSE).
