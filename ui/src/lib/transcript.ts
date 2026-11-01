import type { FailureDocument } from "@/api/client";
import type { Outcome, RunEvent, RunRequest } from "@/api/runs";

/** Where one run is. */
export type RunState =
  | { kind: "starting" }
  | { kind: "running"; startedAt: string; lines: string[] }
  | {
      kind: "ended";
      startedAt: string | null;
      lines: string[];
      exit: number | null;
      durationMs: number;
      outcome: Outcome;
    }
  /** A run whose stream stopped before its end, with the error, if one came. */
  | { kind: "cut"; startedAt: string | null; lines: string[]; words: string | null }
  | { kind: "refused"; problem: FailureDocument | null; words: string }
  | { kind: "broken"; words: string };

/**
 * One card of the transcript. `number` counts from 1 in the order the
 * person started the runs. `line` is the page's own line until the server sends the line it
 * ran.
 */
export type Run = { number: number; request: RunRequest; line: string; state: RunState };

export type Action =
  | { type: "add"; number: number; request: RunRequest; line: string }
  | { type: "event"; number: number; event: RunEvent }
  | { type: "refused"; number: number; problem: FailureDocument | null; words: string }
  | { type: "broken"; number: number; words: string }
  | { type: "closed"; number: number };

/** Applies one action to the transcript, oldest run first. */
export function transcript(runs: Run[], action: Action): Run[] {
  const change = (number: number, next: (run: Run) => Run) =>
    runs.map((run) => (run.number === number ? next(run) : run));
  switch (action.type) {
    case "add": {
      const { number, request, line } = action;
      return [...runs, { number, request, line, state: { kind: "starting" } }];
    }
    case "refused": {
      const { problem, words } = action;
      return change(action.number, (run) => ({
        ...run,
        state: { kind: "refused", problem, words },
      }));
    }
    case "broken": {
      const { words } = action;
      return change(action.number, (run) =>
        run.state.kind === "running"
          ? {
              ...run,
              state: { kind: "cut", startedAt: run.state.startedAt, lines: run.state.lines, words },
            }
          : { ...run, state: { kind: "broken", words } },
      );
    }
    case "event":
      return change(action.number, (run) => applyEvent(run, action.event));
    case "closed":
      return change(action.number, closeRun);
  }
}

/**
 * Applies the end of a run's stream. The `ended` line is always the last
 * line, so a run that is still starting or running was cut off.
 */
function closeRun(run: Run): Run {
  const { state } = run;
  if (state.kind !== "starting" && state.kind !== "running") {
    return run;
  }
  const lines = state.kind === "running" ? state.lines : [];
  const startedAt = state.kind === "running" ? state.startedAt : null;
  return { ...run, state: { kind: "cut", startedAt, lines, words: null } };
}

function applyEvent(run: Run, event: RunEvent): Run {
  const { state } = run;
  const lines = state.kind === "running" ? state.lines : [];
  const startedAt = state.kind === "running" ? state.startedAt : null;
  switch (event.event) {
    case "alive":
      return run;
    case "started":
      return {
        ...run,
        line: event.line,
        state: { kind: "running", startedAt: event.started_at, lines: [] },
      };
    case "line":
      return {
        ...run,
        state: { kind: "running", startedAt: startedAt ?? "", lines: [...lines, event.text] },
      };
    case "ended":
      return {
        ...run,
        state: {
          kind: "ended",
          startedAt,
          lines,
          exit: event.exit,
          durationMs: event.duration_ms,
          outcome: event.outcome,
        },
      };
  }
}
