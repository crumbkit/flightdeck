// One control's page (artboard 57): the rule's state in each repository it
// applies to, the drift of each, why the first failing repository fails and
// how to check it again, what the rule checks, and the groups it is in.
import { counted } from "@crumbkit/ui/a11y";
import { failureDetail, measuredText } from "./cells.ts";
import { CELL_STATES_WORST_FIRST, type ComplianceDocument, type Row } from "./document.ts";
import { type DriftState, driftItems, driftState, driftsOn, driftWord } from "./drift.ts";
import { type GroupEntry, groupEntries, groupsAddress } from "./groups.ts";
import { ruleRows } from "./rules.ts";
import { type Snapshot, snapshotOf } from "./snapshot.ts";
import {
  ageWord,
  andList,
  cellWord,
  checkerField,
  checkerWord,
  rowName,
  shortCommit,
  shortName,
  spokenState,
} from "./words.ts";

/** One repository the control applies to. */
export interface ControlLine {
  readonly name: string;
  readonly state: Row["cells"][number]["state"];
  readonly word: string;
  /** What it measured there, such as "rust-toolchain.toml pins 1.89.0". */
  readonly measured: string;
  readonly run: string | null;
  readonly commit: string | null;
  readonly age: string | null;
  readonly staleReport: boolean;
  /** The repository's drift from the imposed set as a whole. */
  readonly drift: DriftState;
  readonly driftWord: string;
  /** Whether it overrides or lacks this control itself. */
  readonly driftsHere: boolean;
  readonly spoken: string;
}

/** Everything a control's page shows. */
export interface ControlView {
  readonly id: string;
  readonly short: string;
  readonly name: string;
  readonly headline: string;
  /** The first group it is in, which the breadcrumb names, or null. */
  readonly breadcrumb: { readonly id: string; readonly title: string } | null;
  /** Under the path, such as "imposed by platform-standard · in groups rust and toolchain". */
  readonly source: string;
  readonly snapshot: Snapshot;
  readonly repositories: readonly ControlLine[];
  readonly notes: readonly string[];
  readonly notApplied: { readonly names: readonly string[]; readonly toggle: string };
  /** The repositories with no report to read, of which the page cannot tell whether the control applies. */
  readonly noReport: readonly string[];
  readonly failing: {
    readonly repository: string;
    readonly word: string;
    readonly detail: string;
    readonly recheck: { readonly argv: readonly string[]; readonly hint: string };
  } | null;
  readonly checks: readonly { readonly term: string; readonly value: string }[];
  readonly groups: readonly GroupEntry[];
}

const SHOWN_WORDS: Readonly<Record<string, string>> = {
  start: "an agent sees it at the start of each session",
  edit: "an agent sees it before the first edit of a file it covers",
  failing: "an agent sees it only while it fails",
};

/** Returns a sentence's first word in lower case, as "Lacks /rules/x" reads inside a sentence. */
function lowerFirst(text: string): string {
  return `${text.charAt(0).toLowerCase()}${text.slice(1)}`;
}

/** Returns the page of the control `rule`, or null when the document holds no such rule. */
export function controlView(document: ComplianceDocument, rule: string): ControlView | null {
  const row = ruleRows(document).find((each) => each.id === rule);
  if (row === undefined) return null;
  const short = shortName(row.id);
  const lines: (ControlLine & { at: number })[] = [];
  const notApplied: string[] = [];
  const noReport: string[] = [];
  document.repositories.forEach((column, index) => {
    const cell = row.cells[index];
    const readable = column.report === "current" || column.report === "stale";
    if (!readable) {
      noReport.push(column.name);
      return;
    }
    if (cell === undefined || cell.state === "absent") {
      notApplied.push(column.name);
      return;
    }
    const drift = driftState(column);
    const word = cellWord(cell.state, cell.code);
    lines.push({
      at: index,
      name: column.name,
      state: cell.state,
      word,
      measured: measuredText(row, cell),
      run: column.source_run === null ? null : `run #${column.source_run.run_number}`,
      commit: column.commit === null ? null : shortCommit(column.commit),
      age: column.age_s === null ? null : ageWord(column.age_s),
      staleReport: column.report === "stale",
      drift,
      driftWord: driftWord(drift),
      driftsHere: driftsOn(column, row.id),
      spoken: `${column.name}: ${spokenState(cell.state, cell.code)}, ${measuredText(row, cell)}`,
    });
  });
  lines.sort(
    (a, b) =>
      CELL_STATES_WORST_FIRST.indexOf(a.state) - CELL_STATES_WORST_FIRST.indexOf(b.state) ||
      a.at - b.at,
  );
  const repositories = lines.map(({ at: _, ...line }) => line);
  const failingCount = repositories.filter((line) => line.state === "failing").length;
  const staleCount = repositories.filter((line) => line.state === "stale").length;
  const of = counted(repositories.length, "repository", "repositories");
  let headline: string;
  if (failingCount > 0) headline = `${short} fails in ${failingCount} of ${of}`;
  else if (staleCount > 0) headline = `${short} is stale in ${staleCount} of ${of}`;
  else if (row.section === "written") headline = `${short} is stated in ${of} and measured in none`;
  else headline = `${short} is current in every one of ${of}`;
  const notes = document.repositories.flatMap((column) => {
    const reached = repositories.some((line) => line.name === column.name);
    if (!reached || driftState(column) !== "drifts" || driftsOn(column, row.id)) return [];
    const what = andList(driftItems(column).map(lowerFirst));
    return [
      `${column.name} drifts in another rule: it ${what}. It keeps this one as the set says.`,
    ];
  });
  const first = document.repositories
    .map((column, index) => ({ column, cell: row.cells[index] }))
    .find(({ cell }) => cell?.state === "failing");
  const failingDetail = first?.cell === undefined ? null : failureDetail(row, first.cell);
  const firstShort = first === undefined ? "" : shortName(first.column.name);
  const groups = groupEntries(document, groupsAddress.parse("")).filter((group) =>
    row.groups.includes(group.id),
  );
  const titles = groups.map((group) => group.title);
  const add = (checks: { term: string; value: string }[], term: string, value: string | null) => {
    if (value !== null && value !== "") checks.push({ term, value });
  };
  const checks: { term: string; value: string }[] = [];
  add(checks, "Requirement", row.requirement);
  add(checks, "Purpose", row.purpose);
  add(checks, "Checker", checkerWord(row.checker, true));
  const reads = new Set(row.cells.flatMap((cell) => cell.facts?.files ?? []));
  add(checks, "Reads", [...reads].join(", "));
  add(checks, "Capture", checkerField(row.checker, "capture"));
  add(checks, "Kind", row.kind);
  const shown = row.show ?? "edit";
  add(checks, "Shown", `${shown}: ${SHOWN_WORDS[shown] ?? "as its rule says"}`);
  add(
    checks,
    "Source",
    row.source === "imposed"
      ? `Imposed. Named by ${document.imposed.name} in ${document.imposed.from}${document.imposed.commit === null ? "" : ` at ${shortCommit(document.imposed.commit)}`}.`
      : "The repository's own rule.",
  );
  const sourceWords =
    row.source === "imposed" ? `imposed by ${document.imposed.name}` : "a repository's own rule";
  const inGroups = titles.length === 0 ? "in no group" : `in groups ${andList(titles)}`;
  return {
    id: row.id,
    short,
    name: rowName(row),
    headline,
    breadcrumb: groups[0] === undefined ? null : { id: groups[0].id, title: groups[0].title },
    source: `${sourceWords} · ${inGroups}`,
    snapshot: snapshotOf(document),
    repositories,
    notes,
    notApplied: {
      names: notApplied,
      toggle: `Show the ${counted(notApplied.length, "repository", "repositories")} it does not apply to`,
    },
    failing:
      first === undefined || failingDetail === null
        ? null
        : {
            repository: first.column.name,
            word: "failing",
            detail: failingDetail,
            recheck: {
              argv: ["board", "rules", "report", "--path", `/repos/${firstShort}`],
              hint: `Run it in a checkout of ${firstShort}, or re-run the controls job on main. The next read shows the new report.`,
            },
          },
    noReport,
    checks,
    groups,
  };
}
