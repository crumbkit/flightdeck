import type { Argument, Run } from "@/api/model";
import { dayOf, dayWords, percentOf } from "./cost";
import { counted } from "./words";

/**
 * What became of a run, as the usage views colour it: it succeeded,
 * printed a help page, was refused as a usage error, failed, was the same
 * call again, or ran again after a failure.
 */
export type RunKind = "ok" | "help" | "usage" | "fail" | "repeat" | "retry";

/** How close a run must follow the one before it to count as a repeat or a retry, in seconds. */
const repeatWindow = 60;

/** Returns true when two lists of flag names hold the same names in the same order. */
function sameFlags(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((name, index) => name === b[index]);
}

/**
 * Returns the kind of each run, in the order of `runs`, which holds each
 * session's runs in order. A run that succeeded within 60 seconds of a run
 * of the same command in its session is a retry when that run failed, and
 * a repeat when it succeeded with the same flags.
 *
 * The usage log keeps flag names and not their values, so a repeat here
 * is the same command with the same flags, which is looser than the
 * tool's own identical-repeat finding, which compares argument shapes.
 */
export function kindsOf(runs: Run[]): RunKind[] {
  const kinds: RunKind[] = [];
  runs.forEach((run, index) => {
    const before = runs[index - 1];
    const previous = kinds[index - 1];
    const follows =
      before !== undefined &&
      previous !== undefined &&
      before.session === run.session &&
      before.command === run.command &&
      run.at - before.at <= repeatWindow;
    if (run.help) {
      kinds.push("help");
    } else if (run.exit === 2) {
      kinds.push("usage");
    } else if (run.exit !== 0) {
      kinds.push("fail");
    } else if (follows && (previous === "fail" || previous === "usage")) {
      kinds.push("retry");
    } else if (follows && previous !== "help" && sameFlags(before.flags, run.flags)) {
      kinds.push("repeat");
    } else {
      kinds.push("ok");
    }
  });
  return kinds;
}

/** One agent session: its runs in order, their kinds, and its first and last run's time. */
export type Session = {
  number: number;
  caller: string;
  runs: Run[];
  kinds: RunKind[];
  first: number;
  last: number;
};

/** Returns the sessions of `runs`, the one whose last run is newest first. */
export function sessionsOf(runs: Run[]): Session[] {
  const kinds = kindsOf(runs);
  const sessions = new Map<number, Session>();
  runs.forEach((run, index) => {
    const kind = kinds[index] ?? "ok";
    const session = sessions.get(run.session);
    if (session === undefined) {
      sessions.set(run.session, {
        number: run.session,
        caller: run.caller,
        runs: [run],
        kinds: [kind],
        first: run.at,
        last: run.at,
      });
    } else {
      session.runs.push(run);
      session.kinds.push(kind);
      session.first = Math.min(session.first, run.at);
      session.last = Math.max(session.last, run.at);
    }
  });
  return [...sessions.values()].sort((a, b) => b.last - a.last || b.number - a.number);
}

/** Returns a time of day as "10:41" in the browser's time zone. */
function clock(date: Date): string {
  return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

/** Returns a time in seconds as its day, such as "7 Nov". */
function dayAt(seconds: number): string {
  return dayWords(dayOf(new Date(seconds * 1000)));
}

/** Returns the time of day of a time in seconds, with seconds, such as "10:41:24". */
export function timeWords(seconds: number): string {
  const date = new Date(seconds * 1000);
  return `${clock(date)}:${String(date.getSeconds()).padStart(2, "0")}`;
}

/**
 * Returns when a session ran, such as "7 Nov 10:41 to 8 Nov 13:48", naming
 * the second day only when it differs.
 */
export function spanWords(first: number, last: number): string {
  const end = clock(new Date(last * 1000));
  const lastDay = dayAt(last);
  const start = `${dayAt(first)} ${clock(new Date(first * 1000))}`;
  return lastDay === dayAt(first) ? `${start} to ${end}` : `${start} to ${lastDay} ${end}`;
}

/** The shortest pause between two runs that the run list marks, in seconds. */
const markedGap = 10 * 60;

/**
 * Returns the mark between two runs of a session: the new day when the
 * second run falls on another day, the pause when it is 10 minutes or
 * more, such as "20 min" or "2.5 h", and otherwise null.
 */
export function gapWords(before: number, after: number): string | null {
  if (dayAt(before) !== dayAt(after)) {
    return dayAt(after);
  }
  const seconds = after - before;
  if (seconds < markedGap) {
    return null;
  }
  return seconds < 3600 ? `${Math.round(seconds / 60)} min` : `${(seconds / 3600).toFixed(1)} h`;
}

/** The name the steps views give a run: "help" for a help page, else its command. */
export function stepKey(run: Run): string {
  return run.help ? "help" : run.command;
}

/** Returns each step from one run to the next in the same session, by their keys. */
function steps(runs: Run[]): [string, string][] {
  return runs.flatMap((run, index): [string, string][] => {
    const next = runs[index + 1];
    return next === undefined || next.session !== run.session
      ? []
      : [[stepKey(run), stepKey(next)]];
  });
}

/** Returns the counts of `keys`, most first, ties by name. */
function ranked(keys: string[]): [string, number][] {
  const counts = new Map<string, number>();
  for (const key of keys) {
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

/**
 * Which command followed which: `cells[row][column]` counts the steps from
 * `keys[row]` to `keys[column]`. `shown` counts the steps between the keys
 * and `total` every step.
 */
export type Matrix = { keys: string[]; cells: number[][]; shown: number; total: number };

/**
 * Returns the steps between the `limit` commands with the most runs, help
 * pages last among them.
 */
export function stepsMatrix(runs: Run[], limit: number): Matrix {
  const keys = ranked(runs.map(stepKey))
    .slice(0, limit)
    .map(([key]) => key)
    .sort((a, b) => Number(a === "help") - Number(b === "help"));
  const index = new Map(keys.map((key, at) => [key, at]));
  const cells = keys.map(() => keys.map(() => 0));
  const all = steps(runs);
  let shown = 0;
  for (const [from, to] of all) {
    const row = index.get(from);
    const column = index.get(to);
    const cellRow = row === undefined ? undefined : cells[row];
    if (cellRow !== undefined && column !== undefined) {
      cellRow[column] = (cellRow[column] ?? 0) + 1;
      shown += 1;
    }
  }
  return { keys, cells, shown, total: all.length };
}

/** The most steps that the lists beside the matrix show. */
const listed = 6;

/** Returns the commands that ran right after and right before `key`, most first. */
export function neighbours(
  runs: Run[],
  key: string,
): { after: [string, number][]; before: [string, number][] } {
  const all = steps(runs);
  return {
    after: ranked(all.flatMap(([from, to]) => (from === key ? [to] : []))).slice(0, listed),
    before: ranked(all.flatMap(([from, to]) => (to === key ? [from] : []))).slice(0, listed),
  };
}

/** Returns the commands that ran twice in a row, most first. */
export function loops(runs: Run[]): [string, number][] {
  return ranked(steps(runs).flatMap(([from, to]) => (from === to ? [from] : []))).slice(0, listed);
}

/** One flag of a command, how many of its runs passed it, and the note beside it. */
export type FlagRow = {
  name: string;
  runs: number;
  note: string;
  tone: "plain" | "muted" | "bad";
  /** Whether every command takes the flag, as `--json` and `--no-input` are. */
  global: boolean;
};

/** The share of runs from which the note says how many runs passed a flag. */
const mostRuns = 0.7;

/**
 * Returns how many of `command`'s runs passed each flag that `declared`
 * names: the command's own flags in their order, then the global flags,
 * such as `--json`, that a run passed. A flag no run passed, a required flag, a default that
 * runs override and a flag that most runs pass each get a note.
 */
export function flagRows(
  declared: Argument[],
  history: Run[],
  command: string,
): { runs: number; flags: FlagRow[] } {
  const runs = history.filter((run) => run.command === command && !run.help);
  const passed = (name: string) => runs.filter((run) => run.flags.includes(name)).length;
  const named = declared.filter((argument) => argument.kind === "named");
  const own = named.filter((argument) => !argument.global);
  const global = named.filter((argument) => argument.global);
  const rows: FlagRow[] = own.map((argument) => {
    const count = passed(argument.name);
    const row = { name: argument.name, runs: count, global: false };
    if (argument.required) {
      return { ...row, note: "required", tone: "plain" };
    }
    if (count === 0) {
      return { ...row, note: "never passed", tone: "bad" };
    }
    const fallback = argument.defaults.join(", ");
    if (fallback !== "") {
      const percent = percentOf(count, runs.length);
      return {
        ...row,
        note: `overrides the default of ${fallback} on ${percent}% of runs`,
        tone: "muted",
      };
    }
    if (count >= runs.length * mostRuns) {
      return { ...row, note: `on ${count} of ${counted(runs.length, "run")}`, tone: "plain" };
    }
    return { ...row, note: "", tone: "muted" };
  });
  const globals: FlagRow[] = global.flatMap((argument) => {
    const count = passed(argument.name);
    return count === 0
      ? []
      : [{ name: argument.name, runs: count, note: "global flag", tone: "muted", global: true }];
  });
  return { runs: runs.length, flags: [...rows, ...globals] };
}
