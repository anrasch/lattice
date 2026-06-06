import { writable, get } from "svelte/store";
import type { TreeEntry, VaultInfo } from "./api";

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
