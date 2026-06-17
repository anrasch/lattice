import { test, expect, beforeEach } from "vitest";
import { get } from "svelte/store";
import {
  treeEntries,
  collapsed,
  currentNote,
  linksRevision,
  noteRevision,
  changedPaths,
  applyChanges,
} from "./stores";
import type { TreeEntry } from "./api";

function entry(path: string): TreeEntry {
  const i = path.lastIndexOf("/");
  return {
    path,
    dir: i === -1 ? "" : path.slice(0, i),
    name: i === -1 ? path : path.slice(i + 1),
    title: path,
    is_index: false,
  };
}

beforeEach(() => {
  treeEntries.set([entry("a.md"), entry("docs/b.md")]);
  collapsed.set(new Set(["docs"]));
  currentNote.set(null);
  linksRevision.set(0);
  noteRevision.set(0);
  changedPaths.set(new Set());
});

test("applyChanges splices entries and bumps linksRevision", () => {
  applyChanges([{ path: "docs/c.md", entry: entry("docs/c.md") }]);
  expect(get(treeEntries).map((e) => e.path).sort()).toEqual([
    "a.md",
    "docs/b.md",
    "docs/c.md",
  ]);
  expect(get(linksRevision)).toBe(1);
  expect(get(changedPaths).has("docs/c.md")).toBe(true);
});

test("noteRevision bumps only when the current note changed", () => {
  currentNote.set("a.md");
  applyChanges([{ path: "docs/b.md", entry: entry("docs/b.md") }]);
  expect(get(noteRevision)).toBe(0); // current note untouched
  applyChanges([{ path: "a.md", entry: entry("a.md") }]);
  expect(get(noteRevision)).toBe(1); // current note in batch
});

test("small batch reveals ancestor folders of changed paths", () => {
  applyChanges([{ path: "docs/new.md", entry: entry("docs/new.md") }]);
  expect(get(collapsed).has("docs")).toBe(false); // ancestor expanded
});

test("new nested folders are revealed for a small batch", () => {
  applyChanges([{ path: "deep/sub/x.md", entry: entry("deep/sub/x.md") }]);
  const c = get(collapsed);
  expect(c.has("deep")).toBe(false);
  expect(c.has("deep/sub")).toBe(false);
});
