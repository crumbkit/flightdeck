// The runs a fetch named for a repository (artboards 52 to 55): which run a
// page names, and the sentences that say how often the controls job runs,
// when it last ran and that a run uploaded no report. A document built
// without fetch records names no runs, and these return null.
import { counted } from "@crumbkit/ui/a11y";
import type { BranchRun, RepositoryColumn } from "./document.ts";
import { cadenceWord, daysWord, dayWord } from "./words.ts";

/** The run a page names for a repository. */
export interface NamedRun {
  readonly number: number;
  readonly branch: string | null;
  readonly commit: string | null;
  /** The run's page on GitHub. */
  readonly url: string;
}

/** Returns the newest run of a column, or null without a fetch record. */
function newest(column: RepositoryColumn): BranchRun | null {
  return column.newest_run ?? null;
}

/** Whether the newest run of a column finished without uploading a report. */
export function noUpload(column: RepositoryColumn): boolean {
  const run = newest(column);
  return run !== null && !run.uploaded;
}

/**
 * Whether a column reads no report because its newest run uploaded none
 * while an earlier run's report is beside the fetch record. Its cells read
 * no report where that report held the rule, so they say which rules reach
 * it.
 */
export function superseded(column: RepositoryColumn): boolean {
  return column.report === "no_report" && (column.report_run ?? null) !== null;
}

/**
 * Returns the run a page names: the newest run when it uploaded nothing,
 * or the run that made the report.
 */
export function namedRun(column: RepositoryColumn): NamedRun | null {
  const run = newest(column);
  if (run !== null && !run.uploaded) {
    return { number: run.run_number, branch: run.branch, commit: run.commit, url: run.url };
  }
  const source = column.source_run;
  if (source === null) return null;
  const fetched = column.report_run ?? null;
  const same = fetched !== null && fetched.run_id === source.run_id ? fetched : null;
  return {
    number: source.run_number,
    branch: same?.branch ?? run?.branch ?? null,
    commit: source.commit,
    url: same?.url ?? `https://github.com/${source.repository}/actions/runs/${source.run_id}`,
  };
}

/** Returns "Main run #2310", with the branch capitalised, or "Run #2310" without one. */
export function runName(run: NamedRun): string {
  if (run.branch === null) return `Run #${run.number}`;
  return `${run.branch.charAt(0).toUpperCase()}${run.branch.slice(1)} run #${run.number}`;
}

/** The phrase beside a run that uploaded no report. */
export const NO_UPLOAD = "finished without uploading a controls report";

/** Returns "Main run #2310 finished without uploading a controls report.", or null. */
export function noUploadSentence(column: RepositoryColumn): string | null {
  const run = namedRun(column);
  return run === null || !noUpload(column) ? null : `${runName(run)} ${NO_UPLOAD}.`;
}

/**
 * Returns how often a stale report's controls job runs and how old its
 * newest run is, such as "Its controls job runs daily, and the newest main
 * run is 9 days old.", or null for a column that is not stale or names no
 * newest run.
 */
export function staleSentence(column: RepositoryColumn): string | null {
  const run = newest(column);
  if (column.report !== "stale" || run === null || run.age_s === null) return null;
  const age = `the newest ${run.branch} run is ${daysWord(run.age_s)} old.`;
  const hours = column.every_hours ?? null;
  if (hours === null) return `${age.charAt(0).toUpperCase()}${age.slice(1)}`;
  return `Its controls job runs ${cadenceWord(hours)}, and ${age}`;
}

/**
 * Returns when a stale report's controls job last ran, such as "Its
 * controls job runs daily and has not run on main since 16 November.", or
 * null.
 */
export function notRunSentence(column: RepositoryColumn): string | null {
  const run = newest(column);
  if (column.report !== "stale" || run === null) return null;
  const since = `has not run on ${run.branch} since ${dayWord(run.started_at)}.`;
  const hours = column.every_hours ?? null;
  if (hours === null) return `The controls job ${since}`;
  return `Its controls job runs ${cadenceWord(hours)} and ${since}`;
}

/** Returns "3 commits on main since that run", or null when the build was not told the count. */
export function commitsSinceWords(column: RepositoryColumn): string | null {
  const count = column.commits_since ?? null;
  const branch = (column.report_run ?? column.newest_run)?.branch ?? null;
  if (count === null || branch === null) return null;
  if (count === 0) return `no commit on ${branch} since that run`;
  return `${counted(count, "commit")} on ${branch} since that run`;
}
