import { invoke } from "@tauri-apps/api/core";

export type NodeType = "index" | "note";
export interface Node {
  path: string;
  title: string;
  type: NodeType;
}
export interface Edge {
  src: string;
  dst: string | null;
  kind: string;
  raw_target: string;
  resolved: boolean;
}
export interface TreeEntry {
  path: string;
  dir: string;
  name: string;
  title: string;
  is_index: boolean;
}
export interface RawNote {
  content: string;
  hash: string;
}
export type WriteOutcome =
  | { outcome: "written"; hash: string }
  | { outcome: "conflict"; on_disk: string };

export const api = {
  tree: () => invoke<TreeEntry[]>("tree"),
  render: (note: string) => invoke<string>("render", { note }),
  readRaw: (note: string) => invoke<RawNote>("read_raw", { note }),
  save: (note: string, content: string, expectedHash: string) =>
    invoke<WriteOutcome>("save", { note, content, expectedHash }),
  backlinks: (note: string) => invoke<Edge[]>("backlinks", { note }),
  links: (note: string) => invoke<Edge[]>("links", { note }),
  orphans: () => invoke<Node[]>("orphans"),
  brokenLinks: () => invoke<Edge[]>("broken_links"),
  search: (text: string) => invoke<Node[]>("search", { text }),
  query: (filters: string[]) => invoke<Node[]>("query", { filters }),
};
