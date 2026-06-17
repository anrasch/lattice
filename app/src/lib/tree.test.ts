import { test, expect } from "vitest";
import { buildTree, folderPaths, spliceEntries, newFolderPaths } from "./tree";
import type { TreeEntry } from "./api";

function entry(path: string, isIndex = false): TreeEntry {
  const i = path.lastIndexOf("/");
  return {
    path,
    dir: i === -1 ? "" : path.slice(0, i),
    name: i === -1 ? path : path.slice(i + 1),
    title: path,
    is_index: isIndex,
  };
}

test("buildTree nests files under folders, folders before files, alphabetical", () => {
  const tree = buildTree([
    entry("top.md"),
    entry("docs/README.md", true),
    entry("docs/guide.md"),
    entry("docs/sub/deep.md"),
  ]);

  // root: folder `docs` before file `top.md`
  expect(tree.map((n) => n.name)).toEqual(["docs", "top.md"]);

  const docs = tree[0];
  expect(docs.kind).toBe("folder");
  if (docs.kind !== "folder") throw new Error("expected folder");
  // inside docs: folder `sub` first, then files alphabetically (case-insensitive)
  expect(docs.children.map((n) => n.name)).toEqual(["sub", "guide.md", "README.md"]);

  const sub = docs.children[0];
  if (sub.kind !== "folder") throw new Error("expected folder");
  expect(sub.children[0]).toMatchObject({ kind: "file", path: "docs/sub/deep.md" });
});

test("folderPaths returns every ancestor folder once", () => {
  const paths = folderPaths([
    entry("docs/sub/deep.md"),
    entry("docs/guide.md"),
    entry("top.md"),
  ]).sort();
  expect(paths).toEqual(["docs", "docs/sub"]);
});

test("spliceEntries upserts and removes by path", () => {
  const prev = [entry("a.md"), entry("docs/b.md"), entry("docs/c.md")];
  const next = spliceEntries(prev, [
    { path: "docs/b.md", entry: null }, // removed
    { path: "docs/c.md", entry: { ...entry("docs/c.md"), title: "C2" } }, // replaced
    { path: "docs/d.md", entry: entry("docs/d.md") }, // added
  ]);
  const byPath = Object.fromEntries(next.map((e) => [e.path, e]));
  expect(Object.keys(byPath).sort()).toEqual(["a.md", "docs/c.md", "docs/d.md"]);
  expect(byPath["docs/c.md"].title).toBe("C2");
});

test("newFolderPaths returns folders introduced by the next set", () => {
  const prev = [entry("docs/a.md")];
  const next = [entry("docs/a.md"), entry("docs/sub/deep.md"), entry("notes/n.md")];
  expect(newFolderPaths(prev, next).sort()).toEqual(["docs/sub", "notes"]);
});
