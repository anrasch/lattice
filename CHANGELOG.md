# Changelog

All notable changes to Lattice are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project aims to
follow [Semantic Versioning](https://semver.org/) (pre-1.0: minor = features,
patch = fixes).

## [Unreleased]

## [0.3.0] - 2026-06-18

### Added
- **Live updates.** The desktop app is now a live view of the vault instead of an
  open-time snapshot. A filesystem watcher reindexes external changes (git,
  editor saves, MCP/CLI edits) and surgically refreshes the affected tree entries,
  the open note, and the connections panel — no manual reload. A persistent
  "changed on disk" guard protects an in-progress edit (the buffer is never
  clobbered), with a transient "updated" cue, a changed-row highlight, and a
  manual reload (⌘R).
- **Preview tabs.** Single-clicking a note (in the tree, search, orphans, broken,
  or backlinks) opens it in one reused *preview* tab (italic); double-clicking or
  entering edit mode pins it. Browsing no longer piles up tabs.
- **Collapsible connections panel.** The right "Connections" panel collapses to a
  slim edge rail (click to reopen), available even with no note open.
- **`vault_open` — agent-directed open.** A new MCP tool `vault_open` and CLI
  command `lattice open <note>` focus a note in the running desktop app, so an
  agent and the human look at the same document. Delivered over a small control
  file in `~/.lattice` (a `vault://open` event raises the window and opens the
  note in the preview slot). 15 MCP tools total.

## [0.2.0] - 2026-06-06

### Added
- **In-app vault picker.** A Welcome screen with a native folder dialog and a
  recents list; a one-click vault switcher in the sidebar; the last vault is
  remembered and reopened on launch. The bundled app no longer requires
  `LATTICE_ROOT` — startup resolves env → last vault → Welcome. The vault folder
  is never written into (the index db lives in the app cache dir).

### Changed
- **Faster MCP.** The index is built once at startup and kept fresh with cheap
  incremental revalidation, instead of a full rebuild on every tool call.
  Multi-call sessions are roughly 7× faster.

### Fixed
- `vault_patch_frontmatter` no longer corrupts block-style YAML lists or
  CRLF/unparseable frontmatter blocks — it refuses with a warning instead.
- The folder picker no longer deadlocks (the command is async + non-blocking).

## [0.1.0] - 2026-06-05

### Added
- Initial release.
- **Engine (Rust):** parses a markdown vault into a graph (`[[wikilinks]]`,
  markdown links, frontmatter refs, README index nodes); derived SQLite index
  with FTS5; `.aiignore` support.
- **MCP server (15 tools):** `vault_backlinks`, `vault_links`, `vault_query`,
  `vault_search`, `vault_context_bundle`, `vault_index`, `vault_dirs`,
  `vault_keys`, `vault_orphans`, `vault_broken_links`, `vault_changed_since`,
  `vault_superseded`, and structured write-back — `vault_rename` (move a note +
  repair every inbound link) and `vault_patch_frontmatter` (surgical YAML).
  Writes are dry-run by default and never touch git.
- **CLI:** every query and write from the shell.
- **Desktop app (macOS, Tauri + Svelte):** Obsidian-style three-pane workspace,
  rendered read + CodeMirror edit with write-back, light and dark themes.

[Unreleased]: https://github.com/anrasch/lattice/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/anrasch/lattice/releases/tag/v0.2.0
[0.1.0]: https://github.com/anrasch/lattice/releases/tag/v0.1.0
