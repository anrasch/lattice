import type { TreeEntry, ChangedEntry } from "./api";

export interface TreeFolder {
  kind: "folder";
  name: string;
  path: string;
  children: TreeItem[];
}
export interface TreeFile {
  kind: "file";
  name: string;
  path: string;
  isIndex: boolean;
}
export type TreeItem = TreeFolder | TreeFile;

/** Build a nested folder/file tree from the flat vault entries. */
export function buildTree(entries: TreeEntry[]): TreeItem[] {
  const root: TreeFolder = { kind: "folder", name: "", path: "", children: [] };
  const folders = new Map<string, TreeFolder>([["", root]]);

  function ensure(path: string): TreeFolder {
    const existing = folders.get(path);
    if (existing) return existing;
    const i = path.lastIndexOf("/");
    const parent = ensure(i === -1 ? "" : path.slice(0, i));
    const folder: TreeFolder = {
      kind: "folder",
      name: i === -1 ? path : path.slice(i + 1),
      path,
      children: [],
    };
    folders.set(path, folder);
    parent.children.push(folder);
    return folder;
  }

  for (const e of entries) {
    ensure(e.dir).children.push({
      kind: "file",
      name: e.name,
      path: e.path,
      isIndex: e.is_index,
    });
  }

  function sort(f: TreeFolder) {
    f.children.sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "folder" ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    for (const c of f.children) if (c.kind === "folder") sort(c);
  }
  sort(root);
  return root.children;
}

/** Every folder path in the tree (used to default-collapse). */
export function folderPaths(entries: TreeEntry[]): string[] {
  const set = new Set<string>();
  for (const e of entries) {
    const parts = e.dir ? e.dir.split("/") : [];
    let acc = "";
    for (const p of parts) {
      acc = acc ? `${acc}/${p}` : p;
      set.add(acc);
    }
  }
  return [...set];
}

/** Apply a batch of changes to a flat entry list: remove `null`s, upsert the rest. */
export function spliceEntries(
  prev: TreeEntry[],
  changes: ChangedEntry[],
): TreeEntry[] {
  const map = new Map(prev.map((e) => [e.path, e]));
  for (const c of changes) {
    if (c.entry === null) map.delete(c.path);
    else map.set(c.path, c.entry);
  }
  return [...map.values()];
}

/** Folder paths present in `next` but not in `prev` (newly introduced folders). */
export function newFolderPaths(prev: TreeEntry[], next: TreeEntry[]): string[] {
  const before = new Set(folderPaths(prev));
  return folderPaths(next).filter((p) => !before.has(p));
}
