// What the daemon says of a blueprint's text as it checks it: what is
// wrong with it, where, and the graph of what it does. Shapes alone, of the
// daemon's answers; `api.ts` hands them out under the names they had.

import type { Fields, Json } from '../api.ts';

export interface Pos {
  line: number;
  col: number;
}
export interface Diagnostic {
  severity: 'error' | 'warning';
  code: string;
  message: string;
  span: { start: Pos; end: Pos };
}
export interface GraphHook {
  point: string;
  action: string;
  with: Json;
  when: string | null;
}
export interface GraphWatch {
  rule: string;
  action: string;
  with: Json;
  fail: boolean;
}
export interface GraphNode {
  id: string;
  group: string;
  name: string;
  kind: 'action' | 'call' | 'spawn' | 'task' | 'trigger' | 'attachment';
  target: string;
  layer: number;
  line: number;
  end_line: number;
  description: string | null;
  when: string | null;
  for_each: string | null;
  timeout: string | null;
  retry: string | null;
  device: string | null;
  measure: string | null;
  allow_failure: boolean;
  with: Json;
  hooks: GraphHook[];
  watch: GraphWatch[];
}
export interface GraphEdge {
  from: string;
  to: string;
  kind: 'needs' | 'then' | 'fires' | 'starts';
}
export interface GraphParam {
  name: string;
  type: string;
  default: Json;
  required: boolean;
  description: string | null;
  /** The values it may have, when the blueprint says which; otherwise none. */
  choices: Json[];
}
export interface Graph {
  name: string;
  description: string | null;
  params: GraphParam[];
  state: Fields;
  permissions: string[] | null;
  groups: { id: string; label: string; kind: string }[];
  nodes: GraphNode[];
  edges: GraphEdge[];
}
