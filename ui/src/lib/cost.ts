import type { Cost, Finding } from "@/api/model";

/** The findings that count calls an agent could have saved. */
const retry = "retry after failure";
const repeat = "identical repeat";

const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Returns a day written as `YYYY-MM-DD` in words, such as "3 Nov". */
export function dayWords(day: string): string {
  const [, month = "", date = ""] = day.split("-");
  return `${Number(date)} ${months[Number(month) - 1] ?? month}`;
}

/** Returns a count with its thousands grouped, such as "1,384". */
export function grouped(count: number): string {
  return count.toLocaleString("en");
}

/**
 * Returns the card's line under its title: the runs, and the day the
 * figures start when the tool names one, such as "5 runs since 3 Nov".
 */
export function runsWords(runs: number, since: string | null): string {
  const counted = `${grouped(runs)} ${runs === 1 ? "run" : "runs"}`;
  return since === null ? counted : `${counted} since ${dayWords(since)}`;
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
  const tokens = Math.round(bytes / 4);
  return `about ${grouped(tokens)} ${tokens === 1 ? "token" : "tokens"}`;
}

/** Returns `part` as a whole percentage of `whole`, or null when `whole` is 0. */
export function percentOf(part: number, whole: number): number | null {
  return whole === 0 ? null : Math.round((part / whole) * 100);
}

/** One kind of call that an agent could have saved, and how many it made. */
export type Saved = { label: string; count: number };

/** Returns the runs that the findings of `kind` count. */
function runsOf(findings: Finding[], kind: string): number {
  return findings
    .filter((finding) => finding.finding === kind)
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
          { label: "Ran again after a failure", count: runsOf(findings, retry) },
          { label: "Same call again within 60 s", count: runsOf(findings, repeat) },
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

/** A failure code that agents ran the command again after, and how often. */
export type RetriedCode = { code: string; runs: number };

/**
 * Returns the codes of the failures that agents ran the command again
 * after, most retried first, from the findings of a retry after failure.
 */
export function retriedCodes(findings: Finding[] | null): RetriedCode[] {
  const byCode = new Map<string, number>();
  for (const finding of findings ?? []) {
    if (finding.finding === retry && finding.code !== null) {
      byCode.set(finding.code, (byCode.get(finding.code) ?? 0) + finding.runs);
    }
  }
  return [...byCode]
    .map(([code, runs]) => ({ code, runs }))
    .sort((a, b) => b.runs - a.runs || a.code.localeCompare(b.code));
}

/** How a finding's badge reads: a slow command in red, any other in amber. */
export type BadgeTone = "slow" | "waste";

/** Returns the short word of a finding's badge and its tone. */
export function badgeOf(finding: Finding): { label: string; tone: BadgeTone } {
  switch (finding.finding) {
    case "slow command":
      return { label: "slow", tone: "slow" };
    case retry:
      return { label: "retry", tone: "waste" };
    default:
      return { label: finding.finding, tone: "waste" };
  }
}
