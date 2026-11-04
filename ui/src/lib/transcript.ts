import type { FailureDocument } from "@/api/client";
import type { Outcome, RunEvent, RunRequest } from "@/api/runs";
import type { Draft } from "./form";

/** Where one run is. */
export type RunState =
  | { kind: "starting" }
  /** `id` is the id that the cancel route takes, or null when the server gave none. */
  | { kind: "running"; id: string | null; startedAt: string; lines: string[] }
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
  | { kind: "broken"; words: string }
  /** A preview of a command without a dry run, which ran nothing and issued its id. */
  | { kind: "issued" };

/** What the person answers to a preview's question. */
export type Answer = "open" | "declined" | "confirmed";

/**
 * The question of a preview card. `basis` is the composer's draft that the
 * preview was made from, so the card can tell when the composer has
 * changed since, or null for a preview the page opened by itself.
 * `confirmation` is the id the server issued, once it has.
 */
export type Ask = { basis: Draft | null; confirmation: string | null; answer: Answer };

/**
 * One card of the transcript. `number` counts from 1 in the order the
 * person started the runs. `line` is the page's own line until the server sends the line it
 * ran. `long` is true when the person asked for the long columns, which
 * the card's table then shows first. Only a preview card has `ask`.
 */
export type Run = {
  number: number;
  request: RunRequest;
  line: string;
  long: boolean;
  state: RunState;
  ask?: Ask;
};

export type Action =
  | { type: "add"; number: number; request: RunRequest; line: string; long: boolean }
  | {
      type: "preview";
      number: number;
      request: RunRequest;
      line: string;
      long: boolean;
      basis: Draft | null;
    }
  | { type: "answer"; number: number; answer: Exclude<Answer, "open"> }
  /** The events of one run that arrived during one animation frame, in order. */
  | { type: "events"; number: number; events: RunEvent[] }
  | { type: "refused"; number: number; problem: FailureDocument | null; words: string }
  | { type: "broken"; number: number; words: string }
  | { type: "closed"; number: number };

/** Applies one action to the transcript, oldest run first. */
export function transcript(runs: Run[], action: Action): Run[] {
  const change = (number: number, next: (run: Run) => Run) =>
    runs.map((run) => (run.number === number ? next(run) : run));
  switch (action.type) {
    case "add": {
      const { number, request, line, long } = action;
      return [...runs, { number, request, line, long, state: { kind: "starting" } }];
    }
    case "preview": {
      const { number, request, line, long, basis } = action;
      const ask: Ask = { basis, confirmation: null, answer: "open" };
      return [...runs, { number, request, line, long, state: { kind: "starting" }, ask }];
    }
    case "answer":
      return change(action.number, (run) => answered(run, action.answer));
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
    case "events":
      return change(action.number, (run) => action.events.reduce(applyEvent, run));
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

/**
 * Applies the person's answer to a preview card's open question. A
 * confirmed card starts again as the run of the confirmation.
 */
function answered(run: Run, answer: Exclude<Answer, "open">): Run {
  if (run.ask?.answer !== "open") {
    return run;
  }
  const ask = { ...run.ask, answer };
  return answer === "confirmed" ? { ...run, ask, state: { kind: "starting" } } : { ...run, ask };
}

/** Keeps the confirmation id that a preview's event carries, while its question is open. */
function withConfirmation(run: Run, event: RunEvent): Run {
  const id = event.event === "started" || event.event === "ended" ? event.confirmation : undefined;
  if (run.ask?.answer !== "open" || id === undefined || id === null) {
    return run;
  }
  return { ...run, ask: { ...run.ask, confirmation: id } };
}

function applyEvent(run: Run, event: RunEvent): Run {
  return applyRunEvent(withConfirmation(run, event), event);
}

function applyRunEvent(run: Run, event: RunEvent): Run {
  const { state } = run;
  const lines = state.kind === "running" ? state.lines : [];
  const startedAt = state.kind === "running" ? state.startedAt : null;
  const id = state.kind === "running" ? state.id : null;
  switch (event.event) {
    case "alive":
      return run;
    case "started":
      if (event.preview === false) {
        return { ...run, line: event.line, state: { kind: "issued" } };
      }
      return {
        ...run,
        line: event.line,
        state: { kind: "running", id: event.run, startedAt: event.started_at, lines: [] },
      };
    case "line":
      return {
        ...run,
        state: { kind: "running", id, startedAt: startedAt ?? "", lines: [...lines, event.text] },
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
