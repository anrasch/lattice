import { writable } from "svelte/store";
import type { TreeEntry } from "./api";

export const currentNote = writable<string | null>(null);
export const mode = writable<"read" | "edit">("read");
export const treeEntries = writable<TreeEntry[]>([]);
