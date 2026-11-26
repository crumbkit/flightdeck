// The Compliance page's view model (artboards 46, 54 and 56): the headline,
// the tiles, the matrix of rules and checks by repository, the lists of
// failing and stale checks, the repositories without a current report and
// each repository's summary for a phone. The only state it reads besides the
// document is the page address's Show filter.
import { counted } from "@crumbkit/ui/a11y";
import { defineAddress, oneOf } from "@crumbkit/ui/address";
import { failureDetail, staleAge, staleDetail } from "./cells.ts";
import type { ComplianceDocument, RepositoryColumn, Row } from "./document.ts";
import { type ColumnHead, columnHeads, type MatrixRow, matrixRows } from "./matrix.ts";
import { type Snapshot, snapshotOf } from "./snapshot.ts";
import { ageWord, andList, daysWord, shortName } from "./words.ts";

/** The Compliance page's address: `?show=not_current` keeps only rows with a cell that is not current. */
export const complianceAddress = defineAddress({
  show: oneOf(["every", "not_current"], "every"),
});

/** The values the Compliance page's address holds. */
export type ComplianceAddress = ReturnType<typeof complianceAddress.parse>;

/** The words of the Show filter's two chips. */
export const SHOW_WORDS: Readonly<Record<ComplianceAddress["show"], string>> = {
  every: "Every state",
  not_current: "Not current",
};

/** One tile above the matrix. */
export interface Tile {
  readonly key: "failing" | "stale" | "current" | "no_report";
  readonly label: string;
  readonly count: number;
  /** The state's word that the tile's chip shows. */
  readonly word: string;
  /** The list the tile links to, or null for one with no list. */
  readonly target: "failing" | "stale" | "missing" | null;
}

/** One section of the matrix, with its heading. */
export interface MatrixSection {
  readonly key: "measured" | "zero" | "suite";
  readonly heading: string;
  readonly rows: readonly MatrixRow[];
}

/** One item of the Failing or Stale list. */
export interface CheckItem {
  readonly key: string;
  /** The repositories it holds for, which share one sentence. */
  readonly repositories: readonly string[];
  /** The rule's or suite's name, or for a stale report the number of its rules. */
  readonly name: string;
  readonly detail: string;
  /** The run that reported it, such as "Run #301", or null when the report came from no CI run. */
  readonly run: string | null;
  /** The rule's or suite's path the item opens on its repository's page, or null. */
  readonly rule: string | null;
}

/** One repository without a current report. */
export interface MissingItem {
  readonly key: string;
  readonly repository: string;
  /** "no report", "invalid report" or "report 9 d old". */
  readonly word: string;
  readonly detail: string;
  readonly run: string | null;
}

/** One repository's line in the phone's list. */
export interface RepositorySummary {
  readonly name: string;
  /** Such as "1 failing · report 3 h", "all current · report 1 d" or "no report". */
  readonly summary: string;
  /** The word of its worst state. */
  readonly word: string;
  readonly spoken: string;
}

/** Everything the Compliance page shows. */
export interface ComplianceView {
  readonly title: string;
  readonly headline: string;
  readonly snapshot: Snapshot;
  readonly show: ComplianceAddress["show"];
  readonly tiles: readonly Tile[];
  readonly columns: readonly ColumnHead[];
  readonly sections: readonly MatrixSection[];
  /** The written rules, which the matrix shows only once the reader opens them. */
  readonly written: {
    readonly heading: string;
    readonly rows: readonly MatrixRow[];
    /** The words of the button that opens them, such as "Show 15 written rules". */
    readonly toggle: string;
    readonly note: string;
  };
  readonly staleNote: string;
  readonly failing: { readonly count: number; readonly items: readonly CheckItem[] };
  readonly stale: { readonly count: number; readonly items: readonly CheckItem[] };
  readonly missing: { readonly count: number; readonly items: readonly MissingItem[] };
  readonly repositories: readonly RepositorySummary[];
}

const HEADINGS: Readonly<Record<MatrixSection["key"], string>> = {
  measured: "Rules measured in each repository's report",
  zero: "Zero rules, read from a suite's newest run",
  suite: "Suites, from each newest clean full run",
};

/** Returns the run that made a column's report, such as "Run #301", or null. */
export function runLabel(column: RepositoryColumn | undefined): string | null {
  const run = column?.source_run ?? null;
  return run === null ? null : `Run #${run.run_number}`;
}

/** Whether a row holds a cell that is not current, so the Not current filter keeps it. */
function notCurrent(row: MatrixRow): boolean {
  return row.cells.some(
    (cell) => cell.state === "failing" || cell.state === "stale" || cell.state === "not_readable",
  );
}

/** Returns the page's headline, such as "6 checks fail in 5 repositories, 5 are stale, and 1 repository sent no report". */
function headline(document: ComplianceDocument): string {
  const failing = document.counts.failing;
  const stale = document.counts.stale;
  const noReport = document.reports.no_report + document.reports.invalid_report;
  const failingIn = document.repositories.filter((column) => column.counts.failing > 0).length;
  const parts: string[] = [];
  if (failing > 0) {
    parts.push(
      `${counted(failing, "check")} ${failing === 1 ? "fails" : "fail"} in ${counted(failingIn, "repository", "repositories")}`,
    );
  }
  if (stale > 0) parts.push(`${stale} ${stale === 1 ? "is" : "are"} stale`);
  if (noReport > 0) {
    parts.push(`${counted(noReport, "repository", "repositories")} sent no report`);
  }
  const last = parts.at(-1);
  if (last === undefined) return "Every check is current";
  return parts.length === 1 ? last : `${parts.slice(0, -1).join(", ")}, and ${last}`;
}

/** Returns the Failing list: one item per row and sentence, rules first, then suites. */
function failingItems(document: ComplianceDocument, rows: readonly MatrixRow[]): CheckItem[] {
  const items: CheckItem[] = [];
  for (const row of rows) {
    const bySentence = new Map<
      string,
      { repositories: string[]; rule: string; run: string | null }
    >();
    document.repositories.forEach((column, index) => {
      const source = sourceRow(row, column);
      const cell = source?.cells[index];
      if (source === undefined || cell === undefined) return;
      const detail = failureDetail(source, cell);
      if (detail === null) return;
      const entry = bySentence.get(detail);
      if (entry === undefined) {
        bySentence.set(detail, {
          repositories: [column.name],
          rule: source.id,
          run: runLabel(column),
        });
      } else {
        entry.repositories.push(column.name);
      }
    });
    for (const [detail, entry] of bySentence) {
      items.push({
        key: `${row.key} ${entry.repositories.join(",")}`,
        repositories: entry.repositories,
        name: row.name,
        detail,
        run: entry.run,
        rule: entry.rule,
      });
    }
  }
  return items;
}

/** Returns the document row a matrix row's cell in `column` comes from. */
function sourceRow(row: MatrixRow, column: RepositoryColumn): Row | undefined {
  if (row.rows.length === 1) return row.rows[0];
  const own = row.rows.find((each) => each.id === row.id.replace("<repo>", shortOf(column.name)));
  return own ?? row.rows[0];
}

/** Returns a repository's short name, as a suite's path names it. */
function shortOf(name: string): string {
  return name.split("/").at(-1) ?? name;
}

/** Returns the Stale list, oldest first, with a stale report's rules as one item for its repository. */
function staleItems(document: ComplianceDocument, rows: readonly MatrixRow[]): CheckItem[] {
  const items: { item: CheckItem; age: number }[] = [];
  const staleRules = new Map<number, Row[]>();
  for (const row of rows) {
    document.repositories.forEach((column, index) => {
      const source = sourceRow(row, column);
      const cell = source?.cells[index];
      if (source === undefined || cell === undefined || cell.state !== "stale") return;
      const detail = staleDetail(source, cell);
      if (detail === null) {
        const list = staleRules.get(index);
        if (list === undefined) staleRules.set(index, [source]);
        else list.push(source);
        return;
      }
      items.push({
        age: staleAge(source, cell, column),
        item: {
          key: `${row.key} ${column.name}`,
          repositories: [column.name],
          name: row.name,
          detail,
          run: runLabel(column),
          rule: source.id,
        },
      });
    });
  }
  for (const [index, rules] of staleRules) {
    const column = document.repositories[index];
    if (column === undefined) continue;
    const days = column.age_s === null ? "old" : `${daysWord(column.age_s)} old`;
    const verb = rules.length === 1 ? "is" : "are";
    items.push({
      age: column.age_s ?? Number.POSITIVE_INFINITY,
      item: {
        key: `report ${column.name}`,
        repositories: [column.name],
        name: counted(rules.length, "measured rule"),
        detail: `Its report is ${days}, so the ${andList(rules.map((rule) => shortName(rule.id).replaceAll("-", " ")))} ${rules.length === 1 ? "rule" : "rules"} ${verb} stale.`,
        run: runLabel(column),
        rule: null,
      },
    });
  }
  return items.sort((a, b) => b.age - a.age).map((entry) => entry.item);
}

/** Returns the repositories whose report is stale, missing or invalid. */
function missingItems(document: ComplianceDocument): MissingItem[] {
  return document.repositories.flatMap((column) => missingItem(column));
}

/** Returns the item of a repository whose report is stale, missing or invalid, in a list of at most one. */
function missingItem(column: RepositoryColumn): MissingItem[] {
  switch (column.report) {
    case "current":
      return [];
    case "stale": {
      const age = column.age_s ?? 0;
      return [
        {
          key: column.name,
          repository: column.name,
          word: `report ${ageWord(age)} old`,
          detail: `Its newest report was made ${daysWord(age)} before the snapshot.`,
          run: runLabel(column),
        },
      ];
    }
    case "no_report":
      return [
        {
          key: column.name,
          repository: column.name,
          word: "no report",
          detail: "No report was found for it, so its rules and suites have no state.",
          run: null,
        },
      ];
    case "invalid_report": {
      const at = column.field === null ? "" : `, at ${column.field}`;
      const why = column.message === null ? "" : `: ${column.message}`;
      return [
        {
          key: column.name,
          repository: column.name,
          word: "invalid report",
          detail: `Its report is not one this build reads${at}${why}.`,
          run: null,
        },
      ];
    }
  }
}

/** Returns each repository's summary for a phone. */
function summaries(
  document: ComplianceDocument,
  heads: readonly ColumnHead[],
): RepositorySummary[] {
  return document.repositories.map((column, index) => {
    const { failing, stale } = column.counts;
    const head = heads[index];
    let summary: string;
    let word: string;
    if (column.report === "no_report" || column.report === "invalid_report") {
      summary = head?.caption ?? "no report";
      word = summary;
    } else {
      const parts = [
        ...(failing > 0 ? [`${failing} failing`] : []),
        ...(stale > 0 ? [`${stale} stale`] : []),
      ];
      summary = `${parts.length === 0 ? "all current" : parts.join(", ")} · ${head?.caption ?? ""}`;
      if (failing > 0) word = "failing";
      else if (stale > 0 || column.report === "stale") word = "stale";
      else word = "current";
    }
    return { name: column.name, summary, word, spoken: `${column.name}: ${word}, ${summary}` };
  });
}

/** Returns the Compliance page of `document` with the filter `address` holds. */
export function complianceView(
  document: ComplianceDocument,
  address: ComplianceAddress,
): ComplianceView {
  const all = matrixRows(document);
  const shown = address.show === "not_current" ? all.filter(notCurrent) : all;
  const sections = (["measured", "zero", "suite"] as const)
    .map((key) => ({
      key,
      heading: HEADINGS[key],
      rows: shown.filter((row) => row.section === key),
    }))
    .filter((section) => section.rows.length > 0);
  const written = shown.filter((row) => row.section === "written");
  const heads = columnHeads(document);
  const missing = missingItems(document);
  const days = counted(document.stale_after_days, "day");
  return {
    title: document.title,
    headline: headline(document),
    snapshot: snapshotOf(document),
    show: address.show,
    tiles: [
      {
        key: "failing",
        label: "Failing",
        count: document.counts.failing,
        word: "failing",
        target: "failing",
      },
      {
        key: "stale",
        label: "Stale",
        count: document.counts.stale,
        word: "stale",
        target: "stale",
      },
      {
        key: "current",
        label: "Current",
        count: document.counts.current,
        word: "current",
        target: null,
      },
      {
        key: "no_report",
        label: "No report",
        count: document.reports.no_report + document.reports.invalid_report,
        word: "no report",
        target: "missing",
      },
    ],
    columns: heads,
    sections,
    written: {
      heading: "Written rules, judged by an audit",
      rows: written,
      toggle: `Show ${counted(written.length, "written rule")}`,
      note: "These rules have no checker, so no cell gets a state of its own.",
    },
    staleNote: `Stale means a repository's newest report, or a suite's newest clean full run, is older than ${days}. The window is written in the snapshot's document.`,
    failing: { count: document.counts.failing, items: failingItems(document, all) },
    stale: { count: document.counts.stale, items: staleItems(document, all) },
    missing: { count: missing.length, items: missing },
    repositories: summaries(document, heads),
  };
}
