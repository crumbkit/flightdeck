import type { Edge, Graph, GraphNode, Group, Lane } from "@/api/workflows";

// The layout of a workflow graph as a vertical swimlane, as the signed
// mockup draws it: one column per lane, one band per group, and each node
// one rank below every node that reaches it by a forward edge. A
// depth-first walk from the start sets the back edges aside, and those run
// up a channel on the right of the lanes.

/** The nodes and edges that show, with or without the conditional nodes. */
export type Visible = { nodes: GraphNode[]; edges: Edge[] };

/**
 * Returns the nodes and edges that show. With `hideConditional`, each
 * conditional node goes, and each `next` edge into it joins each `next`
 * edge out of it, so its neighbours stay joined.
 */
export function visible(graph: Pick<Graph, "nodes" | "edges">, hideConditional: boolean): Visible {
  if (!hideConditional) {
    return { nodes: graph.nodes, edges: graph.edges };
  }
  const hidden = new Set(graph.nodes.filter((node) => node.conditional).map((node) => node.id));
  let edges = graph.edges.slice();
  for (const id of hidden) {
    const into = edges.filter((edge) => edge.to === id);
    const out = edges.filter((edge) => edge.from === id);
    edges = edges.filter((edge) => edge.to !== id && edge.from !== id);
    for (const a of into) {
      for (const b of out) {
        if (a.kind === "next" && b.kind === "next") {
          const label = a.label ?? b.label;
          edges.push({
            from: a.from,
            to: b.to,
            kind: "next",
            ...(label === undefined ? {} : { label }),
          });
        }
      }
    }
  }
  return { nodes: graph.nodes.filter((node) => !hidden.has(node.id)), edges };
}

/** The order in which the walk follows a node's edges: the main path first. */
const priority: Record<Edge["kind"], number> = {
  next: 0,
  pass: 0,
  skip: 1,
  fail: 2,
  retry: 3,
  loop: 4,
};

/** Each node's rank, and the edges the walk found to run back. */
export type Ranks = { rank: Map<string, number>; back: Set<Edge> };

/**
 * Ranks the nodes. A depth-first walk from each start node, then from
 * every node it missed, marks each edge to a node still on the walk as a
 * back edge. Over the rest, a node's rank is one more than the rank of
 * each node that reaches it, and two nodes of one lane never share a rank.
 */
export function rankNodes(nodes: GraphNode[], edges: Edge[]): Ranks {
  const ids = new Set(nodes.map((node) => node.id));
  const out = new Map<string, Edge[]>();
  for (const edge of edges) {
    if (ids.has(edge.from) && ids.has(edge.to)) {
      out.set(edge.from, [...(out.get(edge.from) ?? []), edge]);
    }
  }
  for (const list of out.values()) {
    list.sort((a, b) => priority[a.kind] - priority[b.kind]);
  }
  const state = new Map<string, "open" | "done">();
  const back = new Set<Edge>();
  const visit = (id: string) => {
    state.set(id, "open");
    for (const edge of out.get(id) ?? []) {
      const seen = state.get(edge.to);
      if (seen === "open") {
        back.add(edge);
      } else if (seen === undefined) {
        visit(edge.to);
      }
    }
    state.set(id, "done");
  };
  for (const node of nodes) {
    if (node.kind === "start") {
      visit(node.id);
    }
  }
  for (const node of nodes) {
    if (!state.has(node.id)) {
      visit(node.id);
    }
  }
  const forward = edges.filter((edge) => !back.has(edge) && ids.has(edge.from) && ids.has(edge.to));
  const waiting = new Map(nodes.map((node) => [node.id, 0]));
  for (const edge of forward) {
    waiting.set(edge.to, (waiting.get(edge.to) ?? 0) + 1);
  }
  const lane = new Map(nodes.map((node) => [node.id, node.lane]));
  const rank = new Map<string, number>();
  const queue = nodes.filter((node) => waiting.get(node.id) === 0).map((node) => node.id);
  const taken = new Set<string>();
  // The loop also reads the ids it pushes onto the queue.
  for (const id of queue) {
    let r = rank.get(id) ?? 0;
    while (taken.has(`${lane.get(id)}:${r}`)) {
      r += 1;
    }
    rank.set(id, r);
    taken.add(`${lane.get(id)}:${r}`);
    for (const edge of forward) {
      if (edge.from !== id) {
        continue;
      }
      rank.set(edge.to, Math.max(rank.get(edge.to) ?? 0, r + 1));
      const left = (waiting.get(edge.to) ?? 0) - 1;
      waiting.set(edge.to, left);
      if (left === 0) {
        queue.push(edge.to);
      }
    }
  }
  return { rank, back };
}

/** Measures the width of `text` set in the CSS font `font`, in pixels. */
export type Measure = (text: string, font: string) => number;

/** Returns how many lines `text` takes at `width` pixels, breaking at spaces. */
function lineCount(text: string, width: number, font: string, measure: Measure): number {
  let count = 1;
  let line = "";
  for (const word of text.split(" ")) {
    const longer = line === "" ? word : `${line} ${word}`;
    if (measure(longer, font) > width && line !== "") {
      count += 1;
      line = word;
    } else {
      line = longer;
    }
  }
  return count;
}

/** A lane's column: its left edge and width. */
export type PlacedLane = Lane & { x: number; w: number };

/** A node's box and rank. */
export type PlacedNode = GraphNode & { x: number; y: number; w: number; h: number; r: number };

/** A group's band: its top and bottom. */
export type Band = Group & { top: number; bottom: number };

export type Layout = {
  nodes: PlacedNode[];
  edges: Edge[];
  back: Set<Edge>;
  lanes: PlacedLane[];
  bands: Band[];
  width: number;
  height: number;
  /** The right edge of the last lane, where the back channel starts. */
  lanesRight: number;
};

export type LayoutOptions = {
  hideConditional: boolean;
  /** The number of live records on a node, which adds a tag row. */
  liveAt: (id: string) => number;
  /** The page's sans family, which the label fonts use. */
  family: string;
  measure: Measure;
};

/** The left gutter, which holds the bands' labels. */
export const gutter = 92;

/** The height of the lane header row. */
export const laneHead = 44;

/** Lays `graph` out as the mockup does. */
export function layout(graph: Graph, options: LayoutOptions): Layout {
  const { nodes, edges } = visible(graph, options.hideConditional);
  const { rank, back } = rankNodes(nodes, edges);
  const labelFont = `500 12.5px ${options.family}`;
  const gateFont = `400 12px ${options.family}`;
  const longest = new Map<string, number>();
  for (const node of nodes) {
    longest.set(
      node.lane,
      Math.max(longest.get(node.lane) ?? 0, options.measure(node.label, labelFont)),
    );
  }
  let x = gutter;
  const lanes = graph.lanes.map((lane) => {
    const w = Math.round(Math.max(112, Math.min(176, 60 + (longest.get(lane.id) ?? 0) / 2.5)));
    const placed = { ...lane, x, w };
    x += w;
    return placed;
  });
  const laneOf = new Map(lanes.map((lane) => [lane.id, lane]));
  const lanesRight = x;
  const placed: PlacedNode[] = nodes.map((node) => {
    const lane = laneOf.get(node.lane);
    const w = (lane?.w ?? 112) - 14;
    const font = node.kind === "gate" ? gateFont : labelFont;
    const tags =
      Number(node.conditional) + Number(!node.blocks) + Number(options.liveAt(node.id) > 0);
    const lines = lineCount(node.label, w - 33, font, options.measure);
    const h = (node.kind === "gate" ? 12 : 16) + lines * 16 + (tags > 0 ? 18 : 0);
    return { ...node, x: (lane?.x ?? gutter) + 7, y: 0, w, h, r: rank.get(node.id) ?? 0 };
  });
  const maxRank = Math.max(0, ...placed.map((node) => node.r));
  const groupsAt = (r: number) =>
    placed
      .filter((node) => node.r === r)
      .map((node) => node.group ?? "")
      .sort()
      .join();
  const top: number[] = [];
  let y = 58;
  for (let r = 0; r <= maxRank; r += 1) {
    if (r > 0 && groupsAt(r) !== groupsAt(r - 1)) {
      y += 16;
    }
    top[r] = y;
    const here = placed.filter((node) => node.r === r);
    const next = placed.filter((node) => node.r === r + 1);
    const [only] = here;
    const [after] = next;
    const tight =
      here.length === 1 &&
      next.length === 1 &&
      only?.kind === "gate" &&
      after?.kind === "gate" &&
      only.lane === after.lane;
    y += Math.max(0, ...here.map((node) => node.h)) + (tight ? 12 : 22);
  }
  for (const node of placed) {
    node.y = top[node.r] ?? 58;
  }
  const backs = edges.filter((edge) => back.has(edge)).length;
  const bands = graph.groups.flatMap((group) => {
    const members = placed.filter((node) => node.group === group.id);
    if (members.length === 0) {
      return [];
    }
    return [
      {
        ...group,
        top: Math.min(...members.map((node) => node.y)) - 10,
        bottom: Math.max(...members.map((node) => node.y + node.h)) + 10,
      },
    ];
  });
  return {
    nodes: placed,
    edges,
    back,
    lanes,
    bands,
    width: lanesRight + 12 + backs * 8,
    height: y + 12,
    lanesRight,
  };
}

/** The corner radius of an edge's turns. */
const turn = 6;

/**
 * Returns the SVG path of `edge` from `a` to `b`. A back edge leaves the
 * bottom right of `a`, runs right to its channel at `channel`, up to `b`
 * and left into `b`'s right side. A forward edge runs down, across below
 * `a` or above `b`, and down into `b`'s top.
 */
export function edgePath(
  edge: Edge,
  a: PlacedNode,
  b: PlacedNode,
  placed: Layout,
  channel: number,
): string {
  const R = turn;
  if (placed.back.has(edge)) {
    const sx = a.x + a.w - 12;
    const sy = a.y + a.h;
    const gy = sy + 9;
    const ty = b.y + Math.min(b.h / 2, 14);
    const tx = b.x + b.w;
    return `M${sx} ${sy} V${gy - R} Q${sx} ${gy} ${sx + R} ${gy} H${channel - R} Q${channel} ${gy} ${channel} ${gy - R} V${ty + R} Q${channel} ${ty} ${channel - R} ${ty} H${tx + 6}`;
  }
  const sx = a.x + a.w / 2;
  const sy = a.y + a.h;
  const tx = b.x + b.w / 2;
  const ty = b.y;
  if (Math.abs(sx - tx) < 1) {
    return `M${sx} ${sy} V${ty - 6}`;
  }
  const blocked = placed.nodes.some(
    (node) => node.lane === a.lane && node.r > a.r && node.r < b.r + 1 && node !== b && node !== a,
  );
  const my = blocked ? sy + 11 : ty - 11;
  const d = tx > sx ? 1 : -1;
  return `M${sx} ${sy} V${my - R} Q${sx} ${my} ${sx + d * R} ${my} H${tx - d * R} Q${tx} ${my} ${tx} ${my + R} V${ty - 6}`;
}

/**
 * Returns the x of each back edge's channel, one 8 px step further right
 * for each, in the order of `placed.edges`.
 */
export function channels(placed: Layout): Map<Edge, number> {
  const out = new Map<Edge, number>();
  let x = placed.lanesRight + 2;
  for (const edge of placed.edges) {
    if (placed.back.has(edge)) {
      x += 8;
      out.set(edge, x);
    }
  }
  return out;
}

/**
 * Returns the order of the run sheet: each group's nodes together at the
 * rank of the group's first node, then by rank.
 */
export function runSheet(nodes: GraphNode[], rank: Map<string, number>): GraphNode[] {
  const first = new Map<string, number>();
  for (const node of nodes) {
    if (node.group !== undefined) {
      first.set(
        node.group,
        Math.min(first.get(node.group) ?? Number.POSITIVE_INFINITY, rank.get(node.id) ?? 0),
      );
    }
  }
  const key = (node: GraphNode) =>
    node.group === undefined ? (rank.get(node.id) ?? 0) : (first.get(node.group) ?? 0);
  return nodes
    .slice()
    .sort((a, b) => key(a) - key(b) || (rank.get(a.id) ?? 0) - (rank.get(b.id) ?? 0));
}
