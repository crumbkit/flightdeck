import type { Cost, Day, Finding } from "@/api/model";
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
 * Returns the card's line under its title: the runs, and the days the
 * figures cover when the tool names the first, such as
 * "86 runs, 27 Oct to 10 Nov".
 */
export function periodWords(runs: number, since: Day | null, today: Day): string {
  const count = counted(runs, "run");
  return since === null ? count : `${count}, ${dayWords(since)} to ${dayWords(today)}`;
}

/** Returns bytes in words, such as "1,384 B". */
export function bytesWords(bytes: number): string {
  return `${grouped(bytes)} B`;
}

/**
 * Returns the tokens that `bytes` of output cost an agent, at about four
 * bytes a token, such as "about 346 tokens".
 */
export function tokensWords(bytes: number): string {
  return `about ${counted(Math.round(bytes / 4), "token")}`;
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
const stripOrder: Finding["kind"][] = [
  "slow_command",
  "help_loop",
  "retry_after_failure",
  "heavy_output",
];

/**
 * Returns the findings that the card's strip phrases, in its order: a slow
 * command, a help loop, the retry after failure with the most runs, and
 * heavy output. Identical repeats, the other retries and usage errors are
 * left out, because the saved-calls tile already counts them, and a kind
 * the page does not know is left out too.
 */
export function stripFindings(findings: Finding[] | null): Finding[] {
  return stripOrder.flatMap((kind) => {
    const ofKind = (findings ?? []).filter((finding) => finding.kind === kind);
    const top = ofKind.reduce<Finding | undefined>(
      (best, each) => (best === undefined || each.runs > best.runs ? each : best),
      undefined,
    );
    return top === undefined ? [] : [top];
  });
}
