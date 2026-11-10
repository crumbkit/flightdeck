import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { getJson } from "./client";

/**
 * Why a help page has no bytes, as the server names it: an exit other than
 * 0, a signal, a page that printed nothing, a run that passed its time
 * limit, one that printed more than 64 KiB, a program that would not
 * start, or a run that never started because the measurement passed its
 * own time limit.
 */
const PageFailure = z.enum([
  "exit",
  "signal",
  "empty",
  "timed_out",
  "too_large",
  "not_started",
  "not_run",
]);
export type PageFailure = z.infer<typeof PageFailure>;

/** One help page: its command path, `[]` for the tool's own, and its bytes or why it has none. */
const Page = z.object({
  path: z.array(z.string()),
  bytes: z.number().int().nonnegative().nullable(),
  failure: PageFailure.nullable(),
  /** The exit code or signal number of a page that failed with one. */
  exit: z.number().int().nullable(),
});
export type Page = z.infer<typeof Page>;

/**
 * The reply of `GET /api/weight`: each distinct help page of the tool's
 * commands, the commands whose pages passed the server's cap of 400, and
 * why nothing was measured, or null.
 */
const WeightReply = z.object({
  pages: z.array(Page),
  left_out: z.number().int().nonnegative(),
  problem: z.string().nullable(),
});
export type WeightReply = z.infer<typeof WeightReply>;

/**
 * The server stops starting help runs after 30 s, so the page waits a
 * little longer than that for the reply.
 */
const measureMs = 35_000;

/**
 * The weight query (APP-10). The server runs every help page once for
 * each request, so the page asks only when the Weight tab first opens and
 * keeps the pages until it reloads.
 */
export function weightQuery() {
  return queryOptions({
    queryKey: ["weight"],
    queryFn: ({ signal }) => getJson("/api/weight", WeightReply, signal, measureMs),
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: Number.POSITIVE_INFINITY,
  });
}
