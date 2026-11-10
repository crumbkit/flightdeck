import type { Cost, Day, Finding, Run, Tool } from "@/api/model";
import { counted } from "./words";

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Returns a day in words, such as "3 Nov". */
export function dayWords(day: Day): string {
  return `${day.day} ${months[day.month - 1]}`;
}

/** Returns the day that `date` falls on in the browser's time zone. */
export function dayOf(date: Date): Day {
  return { year: date.getFullYear(), month: date.getMonth() + 1, day: date.getDate() };
}

/** Returns a count with its thousands grouped, such as "1,384". */
export function grouped(count: number): string {
  return count.toLocaleString("en");
}

/**
 * Returns the card's line under its title: the runs, the sessions when the
 * tool lists its runs, and the days the figures cover when the tool names
 * the first, such as "86 runs in 5 sessions, 27 Oct to 10 Nov".
 */
export function periodWords(
  runs: number,
  since: Day | null,
  today: Day,
  sessions: number | null = null,
): string {
  const count =
    sessions === null
      ? counted(runs, "run")
      : `${counted(runs, "run")} in ${counted(sessions, "session")}`;
  return withPeriod(count, since, today);
}

/**
 * Returns `words` followed by the days from `since` to `today`, such as
 * "86 runs, 27 Oct to 10 Nov", or `words` alone when `since` is null.
 */
export function withPeriod(words: string, since: Day | null, today: Day): string {
  return since === null ? words : `${words}, ${dayWords(since)} to ${dayWords(today)}`;
}

/**
 * A view of the usage page. Figures, Weight and Both are about every
 * command, and Figures opens from the top bar. Steps, Sessions and Flags
 * are about one command, and open from the tab bar or the card's links.
 */
export type UsageTab = "figures" | "weight" | "both" | "steps" | "sessions" | "flags";

/** Returns whether the tool reported figures for any of its commands. */
export function hasFigures(tool: Tool): boolean {
  return tool.commands.some((command) => command.cost !== null);
}

/** How many runs failed with one code. */
export type CodeCount = { code: string; count: number };

/**
 * Returns the sessions that ran `command` and its failures by code, most
 * first, from the tool's runs. Only runs that exited with 1 count, as the
 * figures' `failed` does, so the codes never add up to more than the Failed
 * tile's count; usage errors (exit 2) and help pages are counted apart.
 */
export function commandHistory(
  history: Run[],
  command: string,
): { sessions: number; codes: CodeCount[] } {
  const sessions = new Set<number>();
  const codes = new Map<string, number>();
  for (const run of history) {
    if (run.command !== command) {
      continue;
    }
    sessions.add(run.session);
    if (!run.help && run.exit === 1 && run.code !== null) {
      codes.set(run.code, (codes.get(run.code) ?? 0) + 1);
    }
  }
  const sorted = [...codes].map(([code, count]) => ({ code, count }));
  sorted.sort((a, b) => b.count - a.count || a.code.localeCompare(b.code));
  return { sessions: sessions.size, codes: sorted };
}

/**
 * Returns the command that the usage view opens on: `preferred`, the
 * command in the form, when it ran other than as a help page, else the
 * command that ran most, else `preferred` or "".
 */
export function usageCommand(history: Run[], preferred: string | null): string {
  const counts = new Map<string, number>();
  for (const run of history) {
    if (!run.help) {
      counts.set(run.command, (counts.get(run.command) ?? 0) + 1);
    }
  }
  if (preferred !== null && counts.has(preferred)) {
    return preferred;
  }
  // Ties go to the first name, so the view opens on the same command each time.
  const most = [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))[0];
  return most?.[0] ?? preferred ?? "";
}

/** A row of the card's failures: one code, or the codes past the first three. */
export type CodeRow = CodeCount | { code: null; count: number; others: number };

/** Returns the first three codes and one row that sums the rest, if any. */
export function topCodes(codes: CodeCount[]): CodeRow[] {
  const rest = codes.slice(3);
  const others: CodeRow[] =
    rest.length === 0
      ? []
      : [
          {
            code: null,
            count: rest.reduce((sum, each) => sum + each.count, 0),
            others: rest.length,
          },
        ];
  return [...codes.slice(0, 3), ...others];
}

/** Returns bytes in words, such as "1,384 B". */
export function bytesWords(bytes: number): string {
  return `${grouped(bytes)} B`;
}

/**
 * Returns the tokens that `bytes` cost an agent, at four bytes a token:
 * exactly below 10,000, such as "346 tokens", then in thousands, such as
 * "124k tokens", then in millions with one decimal, such as "2.5M
 * tokens". A count that rounds up to the next unit reads in that unit, so
 * 999,600 tokens read "1.0M tokens".
 */
export function tokenCount(bytes: number): string {
  const count = Math.round(bytes / 4);
  if (count < 10_000) {
    return counted(count, "token");
  }
  const thousands = Math.round(count / 1000);
  if (thousands < 1000) {
    return `${thousands}k tokens`;
  }
  return `${(count / 1_000_000).toFixed(1)}M tokens`;
}

/**
 * Returns the tokens that `bytes` cost an agent, as an estimate, such as
 * "about 346 tokens" or "about 124k tokens".
 */
export function tokensWords(bytes: number): string {
  return `about ${tokenCount(bytes)}`;
}

/** Returns `part` as a whole percentage of `whole`, or null when `whole` is 0. */
export function percentOf(part: number, whole: number): number | null {
  return whole === 0 ? null : Math.round((part / whole) * 100);
}

/** One kind of call that an agent could have saved, and how many it made. */
export type Saved = { label: string; count: number };

/** Returns the runs that the findings of `kind` count. */
function runsOf(findings: Finding[], kind: Finding["kind"]): number {
  return findings
    .filter((finding) => finding.kind === kind)
    .reduce((sum, finding) => sum + finding.runs, 0);
}

/**
 * Returns the calls an agent could have saved, each only when the tool
 * reports it: retries after a failure and identical repeats from the
 * findings, and help read first, help pages and usage errors from the
 * figures.
 */
export function savedCalls(cost: Cost): Saved[] {
  const { figures, findings } = cost;
  const fromFindings: Saved[] =
    findings === null
      ? []
      : [
          { label: "Ran again after a failure", count: runsOf(findings, "retry_after_failure") },
          { label: "Same call again within 60 s", count: runsOf(findings, "identical_repeat") },
        ];
  const fromFigures: [string, number | null][] = [
    ["Read help before running", figures.help_before],
    ["Help pages read", figures.help_pages],
    ["Usage errors", figures.refused],
  ];
  return [
    ...fromFindings,
    ...fromFigures.flatMap(([label, count]) => (count === null ? [] : [{ label, count }])),
  ];
}

/** The kinds the strip phrases, in the order it shows them. */
const stripOrder = ["slow_command", "help_loop", "retry_after_failure", "heavy_output"] as const;

/** A kind of finding that the strip phrases. */
export type PhrasedKind = (typeof stripOrder)[number];

/** A finding of a kind that the strip phrases. */
export type Phrased = Finding & { kind: PhrasedKind };

/**
 * Returns the findings that the card's strip phrases, in its order: a slow
 * command, a help loop, the retry after failure with the most runs, and
 * heavy output. Identical repeats, the other retries and usage errors are
 * left out, because the saved-calls tile already counts them, and a kind
 * the page does not know is left out too.
 */
export function stripFindings(findings: Finding[] | null): Phrased[] {
  return stripOrder.flatMap((kind) => {
    const ofKind = (findings ?? []).flatMap((finding) =>
      finding.kind === kind ? [{ ...finding, kind }] : [],
    );
    const top = ofKind.reduce<Phrased | undefined>(
      (best, each) => (best === undefined || each.runs > best.runs ? each : best),
      undefined,
    );
    return top === undefined ? [] : [top];
  });
}
