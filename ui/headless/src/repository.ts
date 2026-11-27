// One repository's compliance page (artboards 47, 55 and 56): its measured
// rules, zero rules, suites and written rules, the rule the page address
// selects with its detail, the phone's single list and the legend of states.
import { counted } from "@crumbkit/ui/a11y";
import { defineAddress, id } from "@crumbkit/ui/address";
import { fileName, measuredPhrase, measuredText } from "./cells.ts";
import { runLabel } from "./compliance.ts";
import {
  CELL_STATES_WORST_FIRST,
  type Cell,
  type CellState,
  type ComplianceDocument,
  type RepositoryColumn,
  type Row,
} from "./document.ts";
import { suiteName } from "./matrix.ts";
import { commitsSinceWords, namedRun, noUploadSentence } from "./runs.ts";
import { type Snapshot, snapshotOf } from "./snapshot.ts";
import {
  ageWord,
  andList,
  cellWord,
  checkerField,
  checkerWord,
  mono,
  type Phrase,
  phrase,
  phraseText,
  rowName,
  shortCommit,
  shortName,
  spokenState,
} from "./words.ts";

/** A repository's page address: `?rule=<path>` selects the rule whose detail the page shows. */
export const repositoryAddress = defineAddress({ rule: id() });

/** The values a repository's page address holds. */
export type RepositoryAddress = ReturnType<typeof repositoryAddress.parse>;

/** One line of a table on the page. */
interface Line {
  /** Unique on the page: the row's path and source. */
  readonly key: string;
  readonly id: string;
  readonly name: string;
  readonly state: CellState;
  readonly word: string;
  readonly spoken: string;
}

/** A measured rule's line: its checker and what it measured. */
export interface RuleLine extends Line {
  readonly checker: string;
  readonly measured: string;
  /** What it measured, with its paths marked for the mono face. */
  readonly measuredPhrase: Phrase;
}

/** A zero rule's line: the suite it reads and that suite's newest run. */
export interface ZeroLine extends Line {
  readonly suite: string;
  readonly measured: string;
  /** What it measured, with its measures marked for the mono face. */
  readonly measuredPhrase: Phrase;
}

/** A suite's line: its newest clean full run and what it found. */
export interface SuiteLine extends Line {
  readonly run: string;
  readonly commitsSince: number | null;
  readonly result: string;
}

/** One line of the phone's single list. */
export interface CheckLine extends Line {
  readonly caption: string;
}

/** The detail of the selected rule or suite. */
export interface Detail {
  readonly key: string;
  readonly id: string;
  /** The panel's heading, such as "Failing · A Rust file stays under 48 KB". */
  readonly label: string;
  readonly word: string;
  /** Each field's term and value, with the value's paths and commits marked for the mono face in `phrase`. */
  readonly fields: readonly {
    readonly term: string;
    readonly value: string;
    readonly phrase: Phrase;
  }[];
  readonly over: readonly { readonly file: string; readonly used: number }[];
  /** The heading of the column of what each file over the limit holds: the budget's unit, such as "Bytes". */
  readonly overHeader: string;
  readonly behind: readonly {
    readonly file: string;
    readonly line: number;
    readonly found: string;
  }[];
  /** The command that makes a new report, and where to run it. */
  readonly recheck: { readonly argv: readonly string[]; readonly hint: string };
}

/** What one state means, for the page's legend. */
export interface LegendEntry {
  readonly word: string;
  readonly meaning: string;
}

/** Everything a repository's page shows. */
export interface RepositoryView {
  readonly name: string;
  readonly headline: string;
  readonly subtitle: string;
  /** Beside the subtitle, such as "3 commits on main since that run"; null when the build was not told the count. */
  readonly commitsSince: string | null;
  readonly snapshot: Snapshot;
  readonly measured: { readonly count: number; readonly rows: readonly RuleLine[] };
  readonly zero: { readonly count: number; readonly rows: readonly ZeroLine[] };
  readonly suites: { readonly count: number; readonly rows: readonly SuiteLine[] };
  readonly written: {
    readonly count: number;
    readonly rows: readonly Line[];
    /** Who judges them, such as "Judged by the Python standard audit, 1 d ago". */
    readonly caption: string;
  };
  readonly checks: readonly CheckLine[];
  readonly selected: Detail | null;
  readonly legend: readonly LegendEntry[];
  readonly readFrom: string;
}

/** The rows that reach a repository, each with its cell there. */
interface Reached {
  readonly row: Row;
  readonly cell: Cell;
}

/** Returns the line fields every table shares. */
function line(column: RepositoryColumn, { row, cell }: Reached): Line {
  const name = row.section === "suite" ? suiteTitle(row.id, column.name) : rowName(row);
  return {
    key: `${row.id} ${row.source ?? "suite"}`,
    id: row.id,
    name,
    state: cell.state,
    word: cellWord(cell.state, cell.code),
    spoken: `${name}: ${spokenState(cell.state, cell.code)}`,
  };
}

/** Returns a repository's suite's name, such as "Rust standard audit" for /suites/api-rust-standard. */
function suiteTitle(path: string, repository: string): string {
  const name = shortName(path);
  const prefix = `${shortName(repository)}-`;
  return suiteName(name.startsWith(prefix) ? name.slice(prefix.length) : name);
}

/** Returns what a suite's run found, such as "1 of 44 cases failed: PY-07". */
function suiteResult(row: Row, cell: Cell): string {
  const run = cell.run;
  if (run === null) return "no clean full run";
  const failed = run.failed.length;
  const cases = run.failed.map((each) => each.case).join(", ");
  const which = failed === 0 ? "" : `: ${cases}`;
  if (row.cases !== null) return `${failed} of ${counted(row.cases, "case")} failed${which}`;
  return failed === 0 ? "no case failed" : `${counted(failed, "case")} failed${which}`;
}

/** Returns a suite's run as the page names it, such as "11 d ago at 8e41d02". */
function runText(cell: Cell): string {
  const run = cell.run;
  if (run === null) return "no clean full run";
  const ago = cell.age_s === null ? "" : `${ageWord(cell.age_s)} ago`;
  const at = run.commit === null ? "" : ` at ${shortCommit(run.commit)}`;
  return `${ago}${at}`.trim();
}

/** Returns the page's headline, such as "owner/app fails 1 rule, and 2 of its checks are stale". */
function headline(column: RepositoryColumn, reached: readonly Reached[]): string {
  if (column.report === "no_report") return `${column.name} sent no report`;
  if (column.report === "invalid_report")
    return `${column.name} sent a report this build cannot read`;
  const failing = reached.filter(({ cell }) => cell.state === "failing");
  const rules = failing.filter(({ row }) => row.section !== "suite").length;
  const audits = failing.length - rules;
  const stale = reached.filter(({ cell }) => cell.state === "stale").length;
  const fails = [
    ...(rules > 0 ? [counted(rules, "rule")] : []),
    ...(audits > 0 ? [counted(audits, "audit")] : []),
  ];
  const staleWords = `${stale} of its checks ${stale === 1 ? "is" : "are"} stale`;
  if (fails.length === 0) {
    return stale === 0
      ? `${column.name} passes every check`
      : `${column.name} fails nothing, and ${staleWords}`;
  }
  const failsWords = `${column.name} fails ${fails.join(" and ")}`;
  return stale === 0 ? failsWords : `${failsWords}, and ${staleWords}`;
}

/** Returns the line under the headline, such as "Report from run #301 at 7f20c4d, 6 h before the snapshot". */
function subtitle(column: RepositoryColumn): string {
  const unsent = noUploadSentence(column);
  if (column.generated === null) return unsent ?? "No report was read for this repository.";
  const named = namedRun(column);
  const branch = named?.branch === null || named?.branch === undefined ? "" : `${named.branch} `;
  const run = named === null ? "" : ` from ${branch}run #${named.number}`;
  const at = column.commit === null ? "" : ` at ${shortCommit(column.commit)}`;
  const age = column.age_s === null ? "" : `, ${ageWord(column.age_s)} before the snapshot`;
  return `Report${run}${at}${age}`;
}

/** Returns the selected rule's or suite's detail. */
function detail(column: RepositoryColumn, reached: Reached): Detail {
  const { row, cell } = reached;
  const name = line(column, reached).name;
  const word = cellWord(cell.state, cell.code);
  const label = `${word.charAt(0).toUpperCase()}${word.slice(1)} · ${name}`;
  const fields: { term: string; value: string; phrase: Phrase }[] = [];
  const add = (term: string, value: Phrase | null) => {
    const text = value === null ? "" : phraseText(value);
    if (value !== null && text !== "") fields.push({ term, value: text, phrase: value });
  };
  const plain = (value: string | null) => (value === null ? null : [value]);
  const code = (value: string | null) => (value === null ? null : [mono(value)]);
  if (row.section === "suite") {
    add("Suite", [row.id]);
    add("Run", cell.run === null ? ["no clean full run"] : [mono(cell.run.id)]);
    add("Result", [suiteResult(row, cell)]);
  } else {
    const files = cell.facts?.files ?? [];
    add("Rule", [row.id]);
    add("Requirement", plain(row.requirement));
    add("Purpose", plain(row.purpose));
    add(
      "Applies to",
      phrase(...(row.paths ?? []).flatMap((glob, at) => [at === 0 ? "" : ", ", mono(glob)])),
    );
    add("Checker", [checkerWord(row.checker, true)]);
    add(
      "Reads",
      phrase(...files.flatMap((file, at) => [at === 0 ? "" : ", ", mono(fileName(file))])),
    );
    add("Capture", code(checkerField(row.checker, "capture")));
  }
  const commit = column.commit === null ? null : shortCommit(column.commit);
  const run = column.source_run === null ? "" : `, in run #${column.source_run.run_number}`;
  add("Measured at", commit === null ? null : phrase(mono(commit), run));
  const short = shortName(column.name);
  const unit = checkerField(row.checker, "unit");
  return {
    key: line(column, reached).key,
    id: row.id,
    label,
    word,
    fields,
    over: cell.facts?.over ?? [],
    overHeader: unit === null ? "Used" : `${unit.charAt(0).toUpperCase()}${unit.slice(1)}`,
    behind: cell.facts?.behind ?? [],
    recheck: {
      argv: ["board", "rules", "report", "--path", `/repos/${short}`],
      hint: `Run it in a checkout of ${short}, or re-run the controls job on main. The next snapshot shows the new report.`,
    },
  };
}

/** Returns `reached` with its worst states first and in the document's order otherwise, as artboard 55's measured table lists them. */
function worstFirst(reached: readonly Reached[]): Reached[] {
  const rank = (each: Reached) => CELL_STATES_WORST_FIRST.indexOf(each.cell.state);
  return [...reached].sort((a, b) => rank(a) - rank(b));
}

/**
 * Returns who judges a repository's written rules: the audits its suites
 * run, such as "Judged by the Python standard audit, 1 d ago", with the run's
 * age when there is one suite.
 */
function judgedBy(column: RepositoryColumn, suites: readonly Reached[]): string {
  const names = suites.map(({ row }) => `the ${suiteTitle(row.id, column.name)}`);
  const only = suites.length === 1 ? suites[0] : undefined;
  if (only === undefined)
    return names.length === 0 ? "Judged by an audit" : `Judged by ${andList(names)}`;
  const age = only.cell.run === null ? null : only.cell.age_s;
  return `Judged by ${names[0] ?? ""}${age === null ? "" : `, ${ageWord(age)} ago`}`;
}

/** Returns the legend of the states the snapshot pages show, with the document's window. */
function legend(document: ComplianceDocument, column: RepositoryColumn): LegendEntry[] {
  const days = counted(document.stale_after_days, "day");
  const branch = (column.newest_run ?? column.report_run)?.branch ?? null;
  return [
    {
      word: "current",
      meaning: `The rule is met in the newest report, or the suite's newest clean run passed within ${days}.`,
    },
    {
      word: "stale",
      meaning: `The newest report or clean run is older than ${days}, or no clean run exists.`,
    },
    {
      word: "failing",
      meaning:
        "A budget is over, a pin is behind, a measure is not 0, or the newest run failed a case.",
    },
    {
      word: "no report",
      meaning:
        branch === null
          ? "No report was found for the repository, so its rules have no state."
          : `The repository's newest ${branch} run uploaded no report, so its rules have no state.`,
    },
  ];
}

/** Returns the page of the repository `name` in `document`, or null when the document has no such repository. */
export function repositoryView(
  document: ComplianceDocument,
  name: string,
  address: RepositoryAddress,
): RepositoryView | null {
  const index = document.repositories.findIndex((column) => column.name === name);
  const column = document.repositories[index];
  if (column === undefined) return null;
  const reached: Reached[] = document.rows.flatMap((row) => {
    const cell = row.cells[index];
    return cell === undefined || cell.state === "absent" ? [] : [{ row, cell }];
  });
  const readable = column.report === "current" || column.report === "stale";
  const of = (section: Row["section"]) =>
    readable ? reached.filter(({ row }) => row.section === section) : [];
  const measured = worstFirst(of("measured")).map(
    (each): RuleLine => ({
      ...line(column, each),
      checker: checkerWord(each.row.checker, true),
      measured: measuredText(each.row, each.cell),
      measuredPhrase: measuredPhrase(each.row, each.cell),
    }),
  );
  const zero = of("zero").map(
    (each): ZeroLine => ({
      ...line(column, each),
      suite: each.cell.facts?.zero?.suite ?? checkerField(each.row.checker, "suite") ?? "–",
      measured: measuredText(each.row, each.cell),
      measuredPhrase: measuredPhrase(each.row, each.cell),
    }),
  );
  const suites = of("suite").map(
    (each): SuiteLine => ({
      ...line(column, each),
      run: runText(each.cell),
      commitsSince: each.cell.run?.commits_since ?? null,
      result: suiteResult(each.row, each.cell),
    }),
  );
  const written = of("written").map((each) => line(column, each));
  const checked = [...of("measured"), ...of("zero"), ...of("suite")];
  const checks = checked
    .map((each, at): CheckLine & { at: number } => {
      const base = line(column, each);
      const caption =
        each.row.section === "suite"
          ? `${runText(each.cell)} · ${suiteResult(each.row, each.cell)}`
          : `${checkerWord(each.row.checker, false)} · ${measuredText(each.row, each.cell)}`;
      return { ...base, caption, at };
    })
    .sort(
      (a, b) =>
        CELL_STATES_WORST_FIRST.indexOf(a.state) - CELL_STATES_WORST_FIRST.indexOf(b.state) ||
        a.at - b.at,
    )
    .map(({ at: _, ...check }) => check);
  const chosen =
    checked.find(({ row }) => row.id === address.rule) ??
    reached.find(({ row }) => readable && row.id === address.rule) ??
    checked.find(({ cell }) => cell.state === "failing") ??
    null;
  const run = runLabel(column);
  return {
    name: column.name,
    headline: headline(column, readable ? reached : []),
    subtitle: subtitle(column),
    commitsSince: readable ? commitsSinceWords(column) : null,
    snapshot: snapshotOf(document),
    measured: { count: measured.length, rows: measured },
    zero: { count: zero.length, rows: zero },
    suites: { count: suites.length, rows: suites },
    written: { count: written.length, rows: written, caption: judgedBy(column, of("suite")) },
    checks,
    selected: chosen === null ? null : detail(column, chosen),
    legend: legend(document, column),
    readFrom: `Read from ${column.name}'s report${run === null ? "" : ` of ${run.toLowerCase()}`}, as it stood at the snapshot.`,
  };
}
