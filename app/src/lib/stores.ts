import { writable, get } from "svelte/store";
import type { TreeEntry, VaultInfo, ChangedEntry } from "./api";
import { spliceEntries, newFolderPaths } from "./tree";

export const vault = writable<VaultInfo | null>(null);
export const currentNote = writable<string | null>(null);
export const mode = writable<"read" | "edit">("read");
export const treeEntries = writable<TreeEntry[]>([]);

export type Theme = "light" | "dark";
export const theme = writable<Theme>("light");

export const openTabs = writable<string[]>([]);
export type LeftView = "files" | "search" | "orphans" | "broken";
export const leftView = writable<LeftView>("files");
export const leftOpen = writable(true);
export const rightOpen = writable(true);
export const collapsed = writable<Set<string>>(new Set());

/** Bumped on every external change/reload; the links panel refetches on it. */
export const linksRevision = writable(0);
/** Bumped only when the open note's content may have changed; NoteView refetches. */
export const noteRevision = writable(0);
/** Paths touched by the latest batch — drives the row highlight; cleared ~2s later. */
export const changedPaths = writable<Set<string>>(new Set());
/** Bumped on every external change — drives the "updated" cue in the vault bar. */
export const externalUpdate = writable(0);

let clearTimer: ReturnType<typeof setTimeout> | undefined;

/** Expand every ancestor folder of `path` (so a changed row is visible). */
function revealAncestors(set: Set<string>, path: string) {
  const parts = path.split("/");
  parts.pop(); // drop the filename
  let acc = "";
  for (const part of parts) {
    acc = acc ? `${acc}/${part}` : part;
    set.delete(acc);
  }
}

/** Apply a `vault://changed` batch to the workspace stores. */
export function applyChanges(changes: ChangedEntry[]) {
  if (!changes.length) return;

  let prev: TreeEntry[] = [];
  treeEntries.update((p) => {
    prev = p;
    return spliceEntries(p, changes);
  });
  const next = get(treeEntries);

  const paths = new Set(changes.map((c) => c.path));
  collapsed.update((s) => {
    const n = new Set(s);
    // New folders default to collapsed...
    for (const f of newFolderPaths(prev, next)) n.add(f);
    // ...but for small batches, reveal what changed.
    if (paths.size <= 5) for (const p of paths) revealAncestors(n, p);
    return n;
  });

  changedPaths.set(paths);
  linksRevision.update((n) => n + 1);
  const cur = get(currentNote);
  if (cur && paths.has(cur)) noteRevision.update((n) => n + 1);
  externalUpdate.update((n) => n + 1);

  if (clearTimer) clearTimeout(clearTimer);
  clearTimer = setTimeout(() => changedPaths.set(new Set()), 2000);
}

/** Open a note (adds a tab if needed) and focus it. */
export function openNote(path: string) {
  const tabs = get(openTabs);
  if (!tabs.includes(path)) openTabs.set([...tabs, path]);
  currentNote.set(path);
  mode.set("read");
}

/** Close a tab, moving focus to a neighbour if it was active. */
export function closeTab(path: string) {
  const tabs = get(openTabs);
  const i = tabs.indexOf(path);
  const next = tabs.filter((t) => t !== path);
  openTabs.set(next);
  if (get(currentNote) === path) {
    currentNote.set(next[Math.min(i, next.length - 1)] ?? null);
  }
}
