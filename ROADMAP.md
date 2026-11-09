# Roadmap

Lattice today (v0.3.0) is a single-user, local tool: a Rust engine that exposes a markdown vault as a graph over MCP, plus a native macOS app that shows live what agents change. This roadmap takes it in three phases from that personal tool to shared memory for a team and its AI agents.

The direction stays the same in every phase: the markdown files are the source of truth, the graph is a derived index, and a human can always see, review and undo what an agent did.

| Phase | Who it serves | Shape |
|---|---|---|
| 1. Local cowork | One person and their agents | Desktop app + local MCP, shared through git |
| 2. Lattice Server | A team on one self-hosted server | Remote MCP over HTTPS, web app, auth, audit trail |
| 3. Lattice Cloud | Many teams | Multi-tenant hosting, permissions, real-time co-editing |

Each phase ships on its own. A later phase builds on the earlier one but never removes the local-first mode.

Priorities used below: **P1** correctness and data safety, **P2** the wins that make it feel real, **P3** depth and polish, **P4** big bets.

---

## Shipped so far

- **Engine:** Rust core with a SQLite/FTS5 index derived from the markdown files, an `rmcp` MCP server and a `clap` CLI.
- **Read tools, dogfooded daily:** search, backlinks, links, frontmatter queries, `dirs`, `keys`, `changed_since`, `superseded`, broken links, orphans, and a token-budgeted context bundle.
- **Structured write-back:** `vault_rename` (move a note and fix every backlink) and `vault_patch_frontmatter`, both dry-run by default. The patcher refuses block-style lists and CRLF files instead of corrupting them (v0.2.0).
- **Long-lived MCP index:** the server builds the index once and runs a cheap incremental sync before each call (about 1.3 s for six calls plus startup, down from about 9 s). Lazy revalidation was chosen over a file watcher for the MCP process, because macOS FSEvents delivered nothing to the watcher thread in a piped-server context, and a silently stale index is the worst failure for a tool whose value is freshness.
- **Desktop app (Tauri 2 + SvelteKit):** read and edit, light and dark, vault picker with recents (v0.2.0), signed and notarized macOS builds.
- **Live updates (v0.3.0):** a watcher in the desktop process reindexes external changes and refreshes only the affected tree entries, open note and connections, with a guard that never overwrites an in-progress edit. Preview tabs and a collapsible connections panel.
- **`vault_open` (v0.3.0):** an agent can open a note in the running app, so the agent and the human look at the same document.

See the [changelog](CHANGELOG.md) for details.

---

## Phase 1: Local cowork

Goal: make the single-user product complete, so a person and their agents can work in the same vault all day without losing work or context. A team can already share a vault in this phase by putting it in a git repo.

Start here, in order of value for effort: 1.1 (merge view), then unlinked mentions and block-style frontmatter from 1.3.

### 1.1 Safe concurrent editing (P2)
1. Replace the "changed on disk" modal, which currently discards the user's edits, with a 3-way merge view (base = content at load, mine = editor buffer, theirs = on-disk) built on `@codemirror/merge`.
2. Accept or reject changes per chunk, and allow editing the merged result before saving.
3. Auto-merge non-overlapping hunks; only show the view for real conflicts.
4. Fix `node_id` so a real database error is no longer reported as "note not found" (P3, tiny: distinguish `QueryReturnedNoRows` from other errors).

Done when: an agent and a human can edit the same note at the same time and nothing is lost silently.

### 1.2 Live activity feed (P3)
The substrate exists: the watcher already emits `vault://changed`, and `vault_open` added `vault://open` and a cross-process control channel. The feed is mostly a new pane on that event stream.
1. Add an activity pane fed by `vault://changed` and `vault://open`: path, change type, short diff summary, time.
2. Mark the origin with a heuristic: edits from this app are "you", external changes are "agent or other tool".
3. Click an entry to open the note with the change highlighted.
4. Keep a session history so a user can review what happened while they were away.
5. UX direction from an earlier multi-session agent controller prototype: a status rail (one colored dot and a one-line live status per agent, like "editing" or "needs approval"), a transcript of structured events where each tool call is a card rather than a terminal scrape, and inline approve, deny or edit cards. Only the UX carries over; the engine stays all-Rust and MCP.

Done when: a user can follow an agent working in the vault without reading the terminal.

### 1.3 Retrieval depth
1. **Section-level retrieval (P3):** index headings, so search and `vault_context_bundle` return the relevant section with `note#heading` provenance instead of whole notes. Deterministic (FTS at heading granularity), not semantic.
2. **Unlinked mentions (P3):** find notes that name another note (by title or path, including backtick paths) without linking to it; expose them as an MCP tool and in the app. This matters most for vaults that cross-reference with `` `path` `` code spans instead of `[[wikilinks]]`, where links, backlinks and orphans are thin today.
3. **Block-style frontmatter and CRLF (P3):** make the frontmatter patcher support block-style lists and CRLF files instead of refusing them.
4. **Schema validation on write (P4):** optional per-type required fields (for example every `type: reference` needs `subject` and `updated`), checked by the write tools. Today the write tools validate graph integrity only.
5. **`dir_summary` in SQL (P4, trivial):** the rollup is O(n) in Rust per call; fine at hundreds of notes, move it into SQL only if it shows up in a profile.

Done when: an agent can pull exactly the paragraphs it needs with citations, and the graph finds the links that plain-path vaults miss.

### 1.4 Editor polish (P3)
1. Typed frontmatter form per document type, instead of editing YAML as raw text.
2. README screenshot and a short demo video.

### 1.5 Distribution (P3)
1. GitHub Actions workflow that builds, signs and attaches release bundles on a `v*` tag.
2. Windows and Linux builds of the desktop app, plus a universal macOS build.
3. Prebuilt CLI and MCP binaries for all three platforms.
4. Document the git-shared vault setup for small teams (one repo, everyone runs Lattice locally).

Done when: a new user on any desktop OS can install Lattice in one step, and a team can share a vault through git.

---

## Phase 2: Lattice Server

Goal: one server hosts a vault for a team. Every team member's agent (Claude, Cursor, any MCP client) connects to it remotely, and every change is attributed and reversible.

### 2.1 Remote MCP
1. Add rmcp's streamable HTTP transport next to stdio, behind a `--http` flag on `lattice-mcp`.
2. Run the server against a vault checkout as a long-lived process with the existing incremental index.
3. Health and readiness endpoints; structured logs.

Done when: an MCP client on another machine can query and edit the vault over HTTPS.

### 2.2 Authentication
1. OAuth 2.1 authorization for the remote endpoint, following the MCP authorization spec.
2. Personal access tokens for scripts and CI.
3. Per-user sessions, so every request carries a user identity.

Done when: only signed-in team members, and their agents, can reach the vault.

### 2.3 Provenance and history
1. Every write records who made it: the user, plus which agent or client acted for them.
2. Every write becomes a git commit on the server, with the user and agent in the commit metadata. History, diff and undo come from git.
3. Feed these records into the activity feed from 1.2, replacing the origin heuristic with real attribution.
4. Revert any single change from the activity feed.

Done when: for any line in the vault, a team can answer "who or which agent wrote this, and when", and roll it back.

### 2.4 Concurrent writers
1. Soft-locks: a note being edited by one user or agent shows as held to others, with a timeout.
2. Server-side merge for non-overlapping edits; conflicts go to the merge view from 1.1.
3. Rate limits per user and per agent.

Done when: several agents and people can write at once without overwriting each other.

### 2.5 Web app
1. Move the desktop app's backend calls (one file, `app/src/lib/api.ts`, about 15 commands) behind an interface with two implementations: Tauri for desktop, HTTP for the web.
2. Serve the same Svelte interface from the server as a web app.
3. Push live updates to browsers over WebSocket or SSE.
4. The desktop app can connect to a remote server as well as to a local folder.

Done when: a team member with only a browser can read, edit and watch the shared vault live.

### 2.6 Deployment
1. Docker image and a `docker compose` example with TLS.
2. Backups: the vault is a git repo, so backup is a push to a second remote; document restore.
3. Admin basics: invite and remove users, rotate tokens, view the audit log.

Done when: a team can self-host Lattice Server on a small VPS in under an hour.

---

## Phase 3: Lattice Cloud

Goal: host many teams, each with their own vaults, without giving up the guarantees of the first two phases. Local-first stays available: any cloud vault can be cloned and used offline.

### 3.1 Multi-tenancy
1. Organizations, workspaces and multiple vaults per workspace.
2. Strict tenant isolation in storage, index and search.
3. Per-tenant quotas and usage metering.

### 3.2 Permissions
1. Roles per workspace (owner, editor, reader).
2. Per-folder permissions, applied to humans and to agents alike (an agent can be limited to read-only, or to one folder).
3. Approval flows: edits from an agent in a protected folder wait as a proposal until a human approves them.

### 3.3 Real-time co-editing
1. Collaborative editing for humans in the web and desktop apps, using a CRDT (Yjs or Automerge) that writes back to the markdown files.
2. Presence: who and which agent is in which note right now.
3. Offline edits that merge on reconnect.

### 3.4 Semantic search
1. Optional embedding index next to FTS5, for hybrid keyword and semantic search.
2. "Question to answer with sources" bundles for agents, always citing note and heading.
3. Keep the deterministic tools as the default; semantic search is opt-in per vault.

### 3.5 Integrations and federation
1. Import from Obsidian, Notion and Confluence exports.
2. Two-way sync with an existing git repository (GitHub, GitLab).
3. Read-only federation: one vault can reference notes in another vault the user has access to.
4. Plugin API for custom MCP tools and app panes.

### 3.6 Operating a service
1. EU hosting by default, data export at any time, account deletion that really deletes.
2. Billing per workspace.
3. Status page, monitoring and incident process.

Done when: a team can sign up, invite people and agents, and work in shared vaults without running a server, and still walk away with plain markdown files and full history whenever they want.

---

## Principles that hold in every phase

- Markdown files are the source of truth. The index can always be rebuilt from them.
- Never manufacture rot: structured edits preview first (dry-run), never touch content they were not asked to change, and refuse rather than corrupt.
- Lattice edits structure (location, frontmatter, links), not prose, and never stages or commits on its own in local mode. Phase 2's server-side commits are an explicit, opt-in server feature.
- Every agent action is visible, attributed and reversible.
- No lock-in: a vault leaves Lattice as a folder of files with its git history.
