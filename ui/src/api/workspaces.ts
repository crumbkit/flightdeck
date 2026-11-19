import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { getJson } from "./client";

// The documents of `flightdeck workspaces list --json --full` and
// `flightdeck workspaces show --json`, which `GET /api/workspaces` and
// `GET /api/workspaces/<name>` send.

/** mxv's failure of one query, or flightdeck's words when mxv printed none. */
const MxvError = z.object({ code: z.string().nullable(), message: z.string() });
export type MxvError = z.infer<typeof MxvError>;

/** Whether `mxv proxy status` answered, and why not when it did not. */
const Routes = z.discriminatedUnion("state", [
  z.object({ state: z.literal("available") }),
  z.object({ state: z.literal("unavailable"), reason: z.string() }),
]);
export type Routes = z.infer<typeof Routes>;

const Count = z.number().int().nonnegative();

/** One workspace as its card draws it. */
const WorkspaceRow = z.object({
  name: z.string().min(1),
  /** The workspace's branch, or null when mxv cannot read its record. */
  branch: z.string().nullable(),
  /** The folder of each checkout. */
  checkouts: z.array(z.string()),
  /** The host port of each named port. */
  ports: z.record(z.string(), z.number().int().positive()),
  /** Why mxv cannot read the workspace's record, or null. */
  problem: z.string().nullable(),
  routes: z.array(z.object({ route: z.string(), url: z.string(), unit: z.string().nullable() })),
  /** The units the manifest declares, or null when `mxv status` failed. */
  declared: Count.nullable(),
  running: Count.nullable(),
  pinned: z.boolean().nullable(),
  units: z.array(z.object({ name: z.string(), state: z.string() })).nullable(),
  /** Why `mxv status` failed for this workspace, or null. */
  status_error: MxvError.nullable(),
});
export type WorkspaceRow = z.infer<typeof WorkspaceRow>;

const WorkspacesList = z.object({
  data: z.array(WorkspaceRow),
  summary: z.object({
    target: z.string(),
    workspaces: Count,
    /** The workspaces with at least one unit running. */
    running: Count,
    /** The workspaces that declare units and have none running. */
    stopped: Count,
    routes: Routes,
  }),
});
export type WorkspacesList = z.infer<typeof WorkspacesList>;

/** One unit as the One workspace page's table draws it. */
const UnitDetail = z.object({
  name: z.string(),
  kind: z.string(),
  state: z.string(),
  ready: z.boolean().nullable(),
  /** Each port as `VARIABLE=port` or `port-name=port`. */
  ports: z.array(z.string()),
  code: z.number().int().nullable(),
  signal: z.number().int().nullable(),
  reason: z.string().nullable(),
  /** The URL of each route that the proxy serves for the unit. */
  routes: z.array(z.string()),
});
export type UnitDetail = z.infer<typeof UnitDetail>;

const CheckoutState = z.object({
  folder: z.string(),
  branch: z.string().nullable(),
  commit: z.string().nullable(),
  /** The files with uncommitted changes, or null when mxv cannot tell. */
  changes: Count.nullable(),
  problem: z.string().nullable(),
});
export type CheckoutState = z.infer<typeof CheckoutState>;

const WorkspaceShow = z.object({
  data: z.object({
    target: z.string(),
    name: z.string(),
    pinned: z.boolean(),
    manifest_error: z.string().nullable(),
    manifest_error_code: z.string().nullable(),
    checkouts: z.array(CheckoutState),
    units: z.array(UnitDetail),
    routes: Routes,
  }),
});
export type WorkspaceShow = z.infer<typeof WorkspaceShow>["data"];

/**
 * How long the page waits for a workspaces reply: the server's query may
 * take its whole 10 seconds, and the reply then has to arrive.
 */
const deadlineMs = 15_000;

/** The Workspaces page's query, asked again every 10 seconds while the tab is visible. */
export function workspacesQuery() {
  return queryOptions({
    queryKey: ["workspaces"],
    queryFn: ({ signal }) => getJson("/api/workspaces", WorkspacesList, signal, deadlineMs),
    refetchInterval: 10_000,
    staleTime: 0,
  });
}

/** The One workspace page's query, asked again every 5 seconds while the tab is visible. */
export function workspaceQuery(name: string) {
  return queryOptions({
    queryKey: ["workspaces", name],
    queryFn: async ({ signal }) =>
      (
        await getJson(
          `/api/workspaces/${encodeURIComponent(name)}`,
          WorkspaceShow,
          signal,
          deadlineMs,
        )
      ).data,
    refetchInterval: 5_000,
    staleTime: 0,
  });
}
