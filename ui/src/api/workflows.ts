import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { getJson } from "./client";

// The documents of `flightdeck workflows list --json --full` and
// `flightdeck workflows show <slug> --json`, which `GET /api/workflows` and
// `GET /api/workflows/<slug>` send. The graph is a
// flightdeck/workflow-graph/v1 record, which the server has checked.

const Count = z.number().int().nonnegative();

/** Whether Board reads the graph, it is degraded, or the check found problems. */
const GraphStatus = z.enum(["ok", "degraded", "problems"]);
export type GraphStatus = z.infer<typeof GraphStatus>;

/** One graph as the Workflows switch names it. */
const GraphRow = z.object({
  slug: z.string(),
  title: z.string().nullable(),
  subject: z.string().nullable(),
  made: z.string().nullable(),
  nodes: Count.nullable(),
  status: GraphStatus,
  code: z.string().nullable(),
  problems: Count,
});
export type GraphRow = z.infer<typeof GraphRow>;

const WorkflowsList = z.object({
  data: z.array(GraphRow),
  summary: z.object({ graphs: Count, drawn: Count }),
});
export type WorkflowsList = z.infer<typeof WorkflowsList>;

/** A place in a source the agent read: a JSON pointer or a line range. */
const Cite = z.discriminatedUnion("at", [
  z.object({ source: Count, at: z.literal("pointer"), pointer: z.string() }),
  z.object({
    source: Count,
    at: z.literal("lines"),
    lines: z.object({ from: z.number().int(), to: z.number().int() }),
  }),
]);
export type Cite = z.infer<typeof Cite>;

const Source = z.object({ ref: z.string(), commit: z.string() });

export const actors = ["owner", "agent", "program", "schedule"] as const;
export type Actor = (typeof actors)[number];

const Lane = z.object({ id: z.string(), label: z.string(), actor: z.enum(actors) });
export type Lane = z.infer<typeof Lane>;

const Group = z.object({
  id: z.string(),
  label: z.string(),
  kind: z.enum(["phase", "loop", "parallel"]),
  parent: z.string().optional(),
  cite: Cite.optional(),
});
export type Group = z.infer<typeof Group>;

export const nodeKinds = ["start", "step", "gate", "choice", "wait", "end"] as const;
export type NodeKind = (typeof nodeKinds)[number];

const Node = z.object({
  id: z.string(),
  label: z.string(),
  kind: z.enum(nodeKinds),
  lane: z.string(),
  group: z.string().optional(),
  detail: z.string().optional(),
  blocks: z.boolean().default(true),
  conditional: z.boolean().default(false),
  when: z.string().optional(),
  cite: Cite.optional(),
  refs: z.array(z.string()).default([]),
});
export type GraphNode = z.infer<typeof Node>;

export const edgeKinds = ["next", "pass", "fail", "retry", "loop", "skip"] as const;
export type EdgeKind = (typeof edgeKinds)[number];

const Edge = z.object({
  from: z.string(),
  to: z.string(),
  kind: z.enum(edgeKinds),
  label: z.string().optional(),
  when: z.string().optional(),
});
export type Edge = z.infer<typeof Edge>;

const Binding = z.object({
  target: z.string(),
  type: z.string(),
  under: z.string(),
  match: z.array(
    z.object({ field: z.string(), op: z.enum(["equals", "under"]), value: z.string() }),
  ),
  version: z.string().optional(),
});
export type Binding = z.infer<typeof Binding>;

/** The graph's schema, which reads a flightdeck/workflow-graph/v1 record. */
export const workflowGraph = z.object({
  title: z.string(),
  summary: z.string(),
  subject: Source,
  sources: z.array(Source),
  lanes: z.array(Lane),
  groups: z.array(Group),
  nodes: z.array(Node),
  edges: z.array(Edge),
  bindings: z.array(Binding),
  made: z.string(),
  by: z.object({
    agent: z.string().optional(),
    person: z.string().optional(),
    program: z.string().optional(),
    session: z.string().optional(),
  }),
});
export type Graph = z.infer<typeof workflowGraph>;

/** A problem the check found, at a JSON pointer into the record. */
const Problem = z.object({ code: z.string(), pointer: z.string(), message: z.string() });
export type Problem = z.infer<typeof Problem>;

/** A live record that a binding placed: its path, title and flow version. */
const Placed = z.object({
  path: z.string(),
  title: z.string(),
  version: z.string().nullable(),
  /** Whether it follows the version that the graph reads. */
  current: z.boolean(),
});
export type Placed = z.infer<typeof Placed>;

/** The records that one binding placed on its node or group. */
const BindingState = z.object({
  target: z.string(),
  records: z.array(Placed),
  /** The records past the 500 that one binding places. */
  left_out: Count,
  /** The records that were degraded or whose field was not text. */
  skipped: Count,
  error: z.object({ code: z.string().nullable(), message: z.string() }).nullable(),
});
export type BindingState = z.infer<typeof BindingState>;

/** Whether what the graph reads has changed since the agent read it. */
const Freshness = z.discriminatedUnion("state", [
  z.object({ state: z.literal("current") }),
  z.object({ state: z.literal("changed") }),
  z.object({ state: z.literal("gone") }),
  z.object({ state: z.literal("unchecked"), reason: z.string() }),
]);
export type Freshness = z.infer<typeof Freshness>;

const WorkflowShow = z.object({
  data: z.object({
    slug: z.string(),
    path: z.string(),
    status: GraphStatus,
    code: z.string().nullable(),
    graph: workflowGraph.nullable(),
    problems: z.array(Problem),
    /** The limit the graph passes, such as "at most 8 lanes", or null. */
    limit: z.string().nullable(),
    bindings: z.array(BindingState),
    /** The number of earlier readings in `board log`, or null. */
    readings: Count.nullable(),
    freshness: Freshness,
    /** The version of the flow that the changes plugin runs, when Board names it. */
    running: z.string().nullable(),
  }),
});
export type WorkflowShow = z.infer<typeof WorkflowShow>["data"];

/**
 * How long the page waits for a workflows reply: the server's queries may
 * take their whole 10 seconds, and the reply then has to arrive.
 */
const deadlineMs = 15_000;

/** The Workflows list, asked again every 30 seconds while the tab is visible. */
export function workflowsQuery() {
  return queryOptions({
    queryKey: ["workflows"],
    queryFn: ({ signal }) => getJson("/api/workflows", WorkflowsList, signal, deadlineMs),
    refetchInterval: 30_000,
    staleTime: 0,
  });
}

/** One graph, asked again every 15 seconds while the tab is visible. */
export function workflowQuery(slug: string) {
  return queryOptions({
    queryKey: ["workflows", slug],
    queryFn: async ({ signal }) =>
      (await getJson(`/api/workflows/${slug}`, WorkflowShow, signal, deadlineMs)).data,
    refetchInterval: 15_000,
    staleTime: 0,
  });
}
