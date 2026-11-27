// The Rules page's view model (artboards 52 and 53): every rule that reaches
// a repository with the state it rolls up to, each repository's report and
// its drift from the imposed set, and the counts by checker, by when an agent
// is shown a rule and by kind. The filters and the sort live in the address.
import { counted } from "@crumbkit/ui/a11y";
import { defineAddress, oneOf } from "@crumbkit/ui/address";
import { filterRows, type RowTest, type Sort, sortRows } from "@crumbkit/ui/table";
import { cellCaption } from "./cells.ts";
import type { Cell, ComplianceDocument, RepositoryColumn, Row } from "./document.ts";
import { addedRules, type DriftState, driftItems, driftState, driftWord } from "./drift.ts";
import {
  NO_UPLOAD,
  namedRun,
  noUpload,
  noUploadSentence,
  staleSentence,
  superseded,
} from "./runs.ts";
import { type Snapshot, snapshotOf } from "./snapshot.ts";
import {
  ageWord,
  checkerWord,
  checkKind,
  INVALID_REPORT,
  NO_REPORT,
  reportWord,
  rowName,
  shortCommit,
} from "./words.ts";

/**
 * The checkers the Checker chips and the Checkers card list, in the order
 * the signed Rules artboard (52) lists them. A zero rule reads a suite's
 * run, and the matrix and a repository's page show it in a section of its
 * own, so the Rules page lists it under All alone.
 */
export const CHECKERS = ["budget", "version", "pattern", "judged", "written"] as const;

/** When an agent is shown a rule, in the order the page lists them. */
export const SHOWN = ["start", "edit", "failing"] as const;

/** The kinds of rule. */
export const KINDS = ["standing", "obligation"] as const;

/** The columns the rules table sorts by. */
const RULE_SORT_KEYS = ["state", "rule"] as const;

/** The column the rules table sorts by. */
export type RuleSortKey = (typeof RULE_SORT_KEYS)[number];

/** The Rules page's address: its four filters and its sort. */
export const rulesAddress = defineAddress({
  kind: oneOf(["all", ...KINDS], "all"),
  checker: oneOf(["all", ...CHECKERS], "all"),
  shown: oneOf(["all", ...SHOWN], "all"),
  state: oneOf(["every", "not_current"], "every"),
  sort: oneOf(RULE_SORT_KEYS, "state"),
  direction: oneOf(["ascending", "descending"], "ascending"),
});

/** The values the Rules page's address holds. */
export type RulesAddress = ReturnType<typeof rulesAddress.parse>;

/** What a rule rolls up to over the repositories it reaches. */
export type RuleState = "failing" | "stale" | "not_readable" | "current" | "stated";

/** The rule states, worst first. */
const RULE_STATES: readonly RuleState[] = ["failing", "stale", "not_readable", "current", "stated"];

/** Returns a rule state's word. */
function ruleStateWord(state: RuleState): string {
  switch (state) {
    case "failing":
      return "failing";
    case "stale":
      return "stale";
    case "not_readable":
      return "not readable";
    case "current":
      return "current";
    case "stated":
      return "stated";
  }
}

/** One rule in the Rules table. */
export interface RuleEntry {
  readonly key: string;
  readonly id: string;
  readonly name: string;
  readonly kind: string;
  readonly checkerKind: string;
  /** Its checker with its limit, such as "budget · words · max 1,000". */
  readonly checker: string;
  readonly shown: string;
  readonly source: "imposed" | "repository";
  /** The repositories it reaches, such as "all 8 with a report", "a, b" or "a and 2 more". */
  readonly repositories: string;
  readonly reached: readonly string[];
  readonly state: RuleState;
  readonly word: string;
  /** Beside the state, such as "web-portal holds 1,212"; null when there is nothing to add. */
  readonly caption: string | null;
  readonly spoken: string;
  readonly groups: readonly string[];
}

/** One repository's line in the Reports table. */
export interface ReportLine {
  readonly name: string;
  /** The CI run that made the report, such as "run #1184", or null for a local report. */
  readonly run: string | null;
  /** The branch the run ran on, such as "main", or null when the build was not told it. */
  readonly branch: string | null;
  /** The run's page on GitHub, or null. */
  readonly runUrl: string | null;
  /**
   * Beside the run: how often a stale report's job runs and how old its
   * newest run is, or that the newest run uploaded no report; null otherwise.
   */
  readonly runNote: string | null;
  /** The phone's line of the same: a full sentence, or null. */
  readonly sentence: string | null;
  readonly commit: string | null;
  readonly age: string | null;
  /** How many rules its report holds, or null without a report. */
  readonly rules: number | null;
  readonly report: RepositoryColumn["report"];
  readonly word: string;
  readonly drift: DriftState;
  readonly driftWord: string;
  readonly driftItems: readonly string[];
  /** The rules it adds for its own app, or null when that is unknown. */
  readonly adds: readonly string[] | null;
  /** A note beside the drift, such as "No report, so its drift is unknown."; null otherwise. */
  readonly note: string | null;
  readonly spoken: string;
}

/** One line of a counts card. */
export interface CountEntry {
  readonly key: string;
  readonly label: string;
  readonly meaning: string;
  readonly count: number;
}

/** Everything the Rules page shows. */
export interface RulesView {
  readonly headline: string;
  readonly snapshot: Snapshot;
  readonly address: RulesAddress;
  readonly sort: Sort<RuleSortKey>;
  readonly tiles: readonly {
    readonly key: RuleState;
    readonly label: string;
    readonly count: number;
  }[];
  readonly imposed: {
    readonly name: string;
    readonly from: string;
    readonly commit: string | null;
    readonly count: number;
    readonly drifting: number;
    readonly sentence: string;
    readonly toggle: string;
    readonly rules: readonly RuleEntry[];
  };
  readonly reports: readonly ReportLine[];
  /** How many repositories have a current report, for the phone's "Reports · 6 of 8 current". */
  readonly reportsCurrent: number;
  /** Every rule after the filters, sorted. */
  readonly rules: readonly RuleEntry[];
  /** How many rules there are before the filters. */
  readonly total: number;
  readonly checkers: readonly CountEntry[];
  readonly shown: readonly CountEntry[];
  readonly kinds: readonly CountEntry[];
  readonly note: string;
}

const CHECKER_MEANINGS: Readonly<Record<(typeof CHECKERS)[number], string>> = {
  budget: "Words or bytes in the files, against a max",
  version: "Every pinned version is at least a minimum",
  pattern: "Text must not match a file of expressions",
  judged: "A model reads text against a rubric",
  written: "No checker; an audit judges the rule",
};

const SHOWN_MEANINGS: Readonly<Record<(typeof SHOWN)[number], string>> = {
  start: "At the start of each session",
  edit: "Before the first edit of a file it covers",
  failing: "Only while its check fails",
};

const KIND_MEANINGS: Readonly<Record<(typeof KINDS)[number], string>> = {
  standing: "Stays until someone changes it",
  obligation: "Work a landed change left owed. It is deleted once that work is committed.",
};

/** Whether a column has a report whose cells say something. */
function readable(column: RepositoryColumn): boolean {
  return column.report === "current" || column.report === "stale";
}

/**
 * Whether a column says which rules reach it: it has a report whose cells
 * say something, or its newest run uploaded none and its cells read no
 * report where an earlier run's report held the rule.
 */
function reaches(column: RepositoryColumn): boolean {
  return readable(column) || superseded(column);
}

/** Returns the caption beside a failing or stale rule, from its first such cell. */
function caption(row: Row, cell: Cell, column: RepositoryColumn): string | null {
  if (cell.state === "stale" && row.section !== "zero" && column.age_s !== null) {
    return `${column.name} report is ${ageWord(column.age_s)} old`;
  }
  const text = cellCaption(row, cell, column);
  if (text === null) return null;
  if (cell.result === "behind") return `${column.name} pins ${text}`;
  if (cell.result === "over" && !text.endsWith(" over")) return `${column.name} holds ${text}`;
  if (cell.result === "over") return `${column.name}: ${text.replace(/ over$/, "")} file over`;
  return `${column.name}: ${text}`;
}

/** Returns the caption of a rule that a report lists and does not measure. */
function statedCaption(row: Row): string {
  switch (checkKind(row.checker)) {
    case "pattern":
      return "applied when a file is evaluated";
    case "judged":
      return "a model reads the text against the rubric";
    default:
      return row.kind === "obligation" ? "owed by a landed change" : "judged by an audit";
  }
}

/**
 * Returns a rule's entry. A column with no report says nothing of which
 * rules reach it, because the build marks its every row not readable, so
 * it counts in neither the rule's reach nor its state. A column whose
 * newest run uploaded no report reaches the rules an earlier run's report
 * held, and makes a measured rule stale when nothing worse holds. The build
 * makes a row only from a rule a report lists, so every rule reaches at
 * least one column.
 */
function entry(document: ComplianceDocument, row: Row): RuleEntry {
  const name = rowName(row);
  const cells = document.repositories.flatMap((column, index) => {
    const cell = row.cells[index];
    return cell === undefined || cell.state === "absent" || !reaches(column)
      ? []
      : [{ column, cell }];
  });
  const reached = cells.map(({ column }) => column.name);
  const withReport = document.repositories.filter(reaches).length;
  const failing = cells.find(({ cell }) => cell.state === "failing");
  const stale = cells.find(({ cell }) => cell.state === "stale");
  const unsent = cells.find(({ column }) => superseded(column));
  const refused = cells.find(
    ({ cell }) =>
      cell.state === "not_readable" &&
      cell.code !== null &&
      cell.code !== NO_REPORT &&
      cell.code !== INVALID_REPORT,
  );
  let state: RuleState;
  let text: string | null = null;
  if (failing !== undefined) {
    state = "failing";
    text = caption(row, failing.cell, failing.column);
  } else if (stale !== undefined) {
    state = "stale";
    text = caption(row, stale.cell, stale.column);
  } else if (unsent !== undefined && row.section !== "written") {
    state = "stale";
    text = `${unsent.column.name} sent no report`;
  } else if (refused !== undefined) {
    state = "not_readable";
    text = refused.cell.code;
  } else if (row.section === "written") {
    state = "stated";
    text = statedCaption(row);
  } else if (cells.some(({ cell }) => cell.state === "not_readable")) {
    state = "not_readable";
  } else {
    state = "current";
  }
  let repositories: string;
  if (reached.length === withReport && reached.length > 1) {
    repositories = `all ${withReport} with a report`;
  } else if (reached.length <= 2) {
    repositories = reached.length === 0 ? "none with a report" : reached.join(", ");
  } else {
    repositories = `${reached[0] ?? ""} and ${reached.length - 1} more`;
  }
  const word = ruleStateWord(state);
  return {
    key: `${row.id} ${row.source ?? ""}`,
    id: row.id,
    name,
    kind: row.kind ?? "standing",
    checkerKind: checkKind(row.checker),
    checker: checkerWord(row.checker, true),
    shown: row.show ?? "edit",
    source: row.source ?? "imposed",
    repositories,
    reached,
    state,
    word,
    caption: text,
    spoken: `${name}: ${word}${text === null ? "" : `, ${text}`}`,
    groups: row.groups,
  };
}

/** Returns each rule once by path, taking the imposed set's row over a repository's copy. */
export function ruleRows(document: ComplianceDocument): Row[] {
  const byId = new Map<string, Row>();
  for (const row of document.rows) {
    if (row.source === null) continue;
    const held = byId.get(row.id);
    if (held === undefined || (held.source === "repository" && row.source === "imposed")) {
      byId.set(row.id, row);
    }
  }
  return [...byId.values()];
}

/** Returns a report's line. */
function reportLine(
  document: ComplianceDocument,
  column: RepositoryColumn,
  index: number,
): ReportLine {
  const has = reaches(column);
  const run = namedRun(column);
  const fresh = staleSentence(column);
  const sha = column.commit ?? (noUpload(column) ? (run?.commit ?? null) : null);
  const rules = has
    ? document.rows.filter((row) => row.source !== null && row.cells[index]?.state !== "absent")
        .length
    : null;
  const drift = driftState(column);
  const word = reportWord(column.report);
  return {
    name: column.name,
    run: run === null ? null : `run #${run.number}`,
    branch: run?.branch ?? null,
    runUrl: run?.url ?? null,
    runNote: noUpload(column) ? NO_UPLOAD : fresh,
    sentence: noUploadSentence(column) ?? fresh,
    commit: sha === null ? null : shortCommit(sha),
    age: column.age_s === null ? null : ageWord(column.age_s),
    rules,
    report: column.report,
    word,
    drift,
    driftWord: driftWord(drift),
    driftItems: driftItems(column),
    adds: addedRules(column),
    note: drift === "unknown" ? "No report, so its drift is unknown." : null,
    spoken: `${column.name}: report ${word}, ${driftWord(drift)} the imposed set`,
  };
}

/** Returns the counts of `keys` among `values`, listing only the keys present. */
function countsOf<K extends string>(
  keys: readonly K[],
  values: readonly string[],
  meanings: Readonly<Record<K, string>>,
): CountEntry[] {
  return keys
    .map((key) => ({
      key,
      label: key,
      meaning: meanings[key],
      count: values.filter((value) => value === key).length,
    }))
    .filter((each) => each.count > 0);
}

/** Returns the Rules page of `document` with the filters and sort `address` holds. */
export function rulesView(document: ComplianceDocument, address: RulesAddress): RulesView {
  const entries = ruleRows(document).map((row) => entry(document, row));
  const tests: RowTest<RuleEntry>[] = [
    (rule) => address.kind === "all" || rule.kind === address.kind,
    (rule) => address.checker === "all" || rule.checkerKind === address.checker,
    (rule) => address.shown === "all" || rule.shown === address.shown,
    (rule) => address.state === "every" || (rule.state !== "current" && rule.state !== "stated"),
  ];
  const sort: Sort<RuleSortKey> = { key: address.sort, direction: address.direction };
  const byState = {
    value: (rule: RuleEntry) => RULE_STATES.indexOf(rule.state),
    direction: sort.direction,
  };
  const byName = { value: (rule: RuleEntry) => rule.name, direction: sort.direction };
  const rules = sortRows(
    filterRows(entries, tests),
    sort.key === "state" ? [byState] : [byName],
    (rule) => rule.name,
  );
  const failing = entries.filter((rule) => rule.state === "failing").length;
  const count = (state: RuleState) => entries.filter((rule) => rule.state === state).length;
  const reports = document.repositories.map((column, index) => reportLine(document, column, index));
  const imposed = entries.filter((rule) => rule.source === "imposed");
  const drifting = reports.filter((report) => report.drift === "drifts").length;
  const commit = document.imposed.commit === null ? null : shortCommit(document.imposed.commit);
  const at = commit === null ? "" : ` at ${commit}`;
  const repositories = counted(document.repositories.length, "repository", "repositories");
  return {
    headline: `${counted(entries.length, "rule")} reach ${repositories}, and ${failing} of them ${failing === 1 ? "fails" : "fail"}`,
    snapshot: snapshotOf(document),
    address,
    sort,
    tiles: [
      { key: "failing", label: "Failing", count: count("failing") },
      { key: "stale", label: "Stale", count: count("stale") },
      { key: "current", label: "Current", count: count("current") },
      { key: "stated", label: "Stated, not measured", count: count("stated") },
    ],
    imposed: {
      name: document.imposed.name,
      from: document.imposed.from,
      commit,
      count: imposed.length,
      drifting,
      sentence: `Every repository follows these ${counted(imposed.length, "rule")}, named by ${document.imposed.name} in ${document.imposed.from}${at}. A repository adds only the rules its own app needs. ${drifting} of ${repositories} drift: they lack or override an imposed rule.`,
      toggle: `Show the ${counted(imposed.length, "imposed rule")}`,
      rules: imposed,
    },
    reports,
    reportsCurrent: reports.filter((report) => report.report === "current").length,
    rules,
    total: entries.length,
    checkers: countsOf(
      CHECKERS,
      entries.map((rule) => rule.checkerKind),
      CHECKER_MEANINGS,
    ),
    shown: countsOf(
      SHOWN,
      entries.map((rule) => rule.shown),
      SHOWN_MEANINGS,
    ),
    kinds: countsOf(
      KINDS,
      entries.map((rule) => rule.kind),
      KIND_MEANINGS,
    ),
    note: "A measured rule fails when any repository's report fails it, and is stale when a repository it reaches has a stale report or none. A rule a report could not read is in no tile. Pattern, judged and written rules are stated: a report lists them and measures nothing.",
  };
}
