import type { Graph, Placed, WorkflowShow } from "@/api/workflows";

/** What the page has selected: a node or a group, by its id. */
export type Selection = { kind: "node" | "group"; id: string } | null;

/**
 * Returns `shown` with each change's `current` taken against the running
 * version of the workflow when the server names one, so every change not
 * on the running version counts as on an older flow, including those on
 * the version the graph reads. A change whose binding names no version
 * stays current. With no running version, `shown` is returned as it is.
 */
export function againstRunning(shown: WorkflowShow): WorkflowShow {
  const running = shown.running;
  if (running === null) {
    return shown;
  }
  return {
    ...shown,
    bindings: shown.bindings.map((binding) => ({
      ...binding,
      records: binding.records.map((record) => ({
        ...record,
        current: record.version === null || record.version === running,
      })),
    })),
  };
}

/**
 * Returns the short label of a graph's title in a phone's switch: the
 * title without a trailing " flow", and of a longer title only its first
 * and last words, so "Weekly suite loop" reads "Weekly loop".
 */
export function phoneLabel(title: string): string {
  const words = title.replace(/ flow$/, "").split(" ");
  return words.length > 2 ? `${words[0]} ${words[words.length - 1]}` : words.join(" ");
}

/**
 * Returns the phone switch's label of each title in `titles`: its short
 * label, or the full title when another title has the same short label.
 */
export function phoneLabels(titles: string[]): string[] {
  const labels = titles.map(phoneLabel);
  return labels.map((label, index) =>
    labels.indexOf(label) === labels.lastIndexOf(label) ? label : (titles[index] ?? label),
  );
}

/** Returns the live records on `target`, the current ones first. */
export function liveOf(shown: WorkflowShow, target: string): Placed[] {
  return shown.bindings
    .filter((binding) => binding.target === target)
    .flatMap((binding) => binding.records)
    .sort((a, b) => Number(!a.current) - Number(!b.current));
}

/** Returns the lane ids whose actor is the owner. */
function ownerLanes(graph: Graph): Set<string> {
  return new Set(graph.lanes.filter((lane) => lane.actor === "owner").map((lane) => lane.id));
}

/** Returns whether `node` is a wait in an owner's lane, which the page fills. */
export function waitsOnYou(graph: Graph, node: { kind: string; lane: string }): boolean {
  return node.kind === "wait" && ownerLanes(graph).has(node.lane);
}

/**
 * Returns the sentence after the summary: the graph's parts, checks and
 * waits on the owner, and how many changes are open on it.
 */
export function headline(graph: Graph, shown: WorkflowShow): string {
  const waits = graph.nodes.filter((node) => waitsOnYou(graph, node)).length;
  const gates = graph.nodes.filter((node) => node.kind === "gate").length;
  const parts = graph.groups.length;
  const start = `${parts} ${parts === 1 ? "part" : "parts"}, ${gates} checks and ${waits} ${waits === 1 ? "step waits" : "steps wait"} on you`;
  if (graph.bindings.length === 0) {
    return `${start}. This graph binds no live records.`;
  }
  const live = shown.bindings.flatMap((binding) => binding.records);
  const old = live.filter((record) => !record.current).length;
  return `${start}. ${live.length} ${live.length === 1 ? "change is" : "changes are"} open on this flow${old > 0 ? `, ${old} of them on an older version` : ""}.`;
}

/**
 * Returns what the address's `selected` names: a node by its id, a group
 * by its id, or the node or group whose binding placed the record at that
 * path. A node wins over a group of the same id.
 */
export function selectionOf(graph: Graph, shown: WorkflowShow, selected: string | null): Selection {
  if (selected === null) {
    return null;
  }
  const target =
    shown.bindings.find((binding) => binding.records.some((record) => record.path === selected))
      ?.target ?? selected;
  if (graph.nodes.some((node) => node.id === target)) {
    return { kind: "node", id: target };
  }
  if (graph.groups.some((group) => group.id === target)) {
    return { kind: "group", id: target };
  }
  return null;
}

/** Returns `selection` while its id still names a node or group of `graph`. */
export function stillThere(graph: Graph, selection: Selection): Selection {
  if (selection === null) {
    return null;
  }
  const ids = selection.kind === "node" ? graph.nodes : graph.groups;
  return ids.some((each) => each.id === selection.id) ? selection : null;
}
