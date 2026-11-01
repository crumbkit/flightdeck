import { z } from "@/api/zod";
import { readLines } from "@/lib/ndjson";
import { ApiError, FailureDocument, postStream } from "./client";
import type { CommandPath } from "./model";

/**
 * The value of one argument as the page sends it: `true` for a flag that is
 * on, a string for a single value and a list of strings for a repeated one.
 */
export type Value = true | string | string[];

/**
 * A run request: the command's path and a value for each argument the
 * person set, by the argument's name in the model, such as `--limit` or
 * `<ID>`. The page never sends a command line as text.
 */
export type RunRequest = { path: CommandPath; values: Record<string, Value> };

const Outcome = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("result") }),
  z.object({ kind: z.literal("result_with_failures") }),
  z.object({ kind: z.literal("tool_failure"), failure: FailureDocument }),
  z.object({ kind: z.literal("cancelled") }),
  z.object({ kind: z.literal("failed"), failure: FailureDocument }),
]);
export type Outcome = z.infer<typeof Outcome>;

/** One line of a run's stream. */
export const RunEvent = z.discriminatedUnion("event", [
  z.object({
    event: z.literal("started"),
    run: z.string().nullable(),
    line: z.string(),
    started_at: z.string(),
  }),
  z.object({ event: z.literal("line"), text: z.string() }),
  // The server writes this during a quiet stretch of a run, and the page ignores it.
  z.object({ event: z.literal("alive") }),
  z.object({
    event: z.literal("ended"),
    outcome: Outcome,
    exit: z.number().int().nullable(),
    duration_ms: z.number().nonnegative(),
  }),
]);
export type RunEvent = z.infer<typeof RunEvent>;

/** Parses one line of a run's stream, or throws ApiError for a line the page cannot read. */
export function parseEvent(line: string): RunEvent {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch (error) {
    throw new ApiError(
      { kind: "shape", issues: "a line of the run is not JSON" },
      { cause: error },
    );
  }
  const parsed = RunEvent.safeParse(value);
  if (!parsed.success) {
    throw new ApiError({ kind: "shape", issues: parsed.error.message }, { cause: parsed.error });
  }
  return parsed.data;
}

/** What one run needs: the request, and where each event of its stream goes. */
export type RunCall = {
  request: RunRequest;
  onEvent: (event: RunEvent) => void;
  signal: AbortSignal;
};

/**
 * Starts a run at `POST /api/runs` and passes each event of its stream to
 * `onEvent` as it arrives. It resolves when the stream closes, whether or
 * not an `ended` line came, and throws ApiError when the server refuses the
 * request or the stream breaks.
 */
export async function streamRun({ request, onEvent, signal }: RunCall): Promise<void> {
  const body = await postStream("/api/runs", request, signal);
  for await (const line of readLines(body)) {
    onEvent(parseEvent(line));
  }
}
