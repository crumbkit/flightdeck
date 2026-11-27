// The matrix of rules and checks by repository that the Compliance page
// draws (artboards 46 and 54): one row per rule, one row per suite that each
// repository runs as its own copy, and a cell per repository.

import { cellCaption } from "./cells.ts";
import {
  CELL_STATES_WORST_FIRST,
  type Cell,
  type ComplianceDocument,
  type RepositoryColumn,
  type Row,
  type Section,
} from "./document.ts";
import {
  ageWord,
  cellWord,
  checkerWord,
  reportWord,
  rowName,
  shortName,
  spokenState,
} from "./words.ts";

/** One column's heading: the repository and the age of its report. */
export interface ColumnHead {
  readonly name: string;
  /** Under the name: "report 3 h", "no report" or "invalid report". */
  readonly caption: string;
  /** Whether the report is older than the window. */
  readonly stale: boolean;
  readonly spoken: string;
}

/** One cell of the matrix. */
export interface MatrixCell {
  readonly repository: string;
  readonly state: Cell["state"];
  /** The chip's word, such as "failing", "no report" or "–" for a rule that does not apply. */
  readonly word: string;
  /** Under the chip, such as "1,184", "2 left" or "2 d · +6"; null when the chip says it all. */
  readonly caption: string | null;
  /** What a screen reader says for the cell: its repository, its row, its state and its caption. */
  readonly spoken: string;
}

/** One row of the matrix. */
export interface MatrixRow {
  /** Unique among the rows: the rule's path and source, or the suite's pattern. */
  readonly key: string;
  /** The rule's path, or the suite's path with `<repo>` for each repository's name. */
  readonly id: string;
  readonly section: Section;
  readonly name: string;
  /** What checks it, such as "budget · words" or "<repo>-rust-standard". */
  readonly checker: string;
  /** The document's rows the matrix row is built from: one for a rule, one per repository for a suite. */
  readonly rows: readonly Row[];
  readonly cells: readonly MatrixCell[];
}

/** Returns the heading of each column. */
export function columnHeads(document: ComplianceDocument): ColumnHead[] {
  return document.repositories.map((column) => {
    const caption =
      column.age_s !== null && (column.report === "current" || column.report === "stale")
        ? `report ${ageWord(column.age_s)}`
        : reportWord(column.report);
    const stale = column.report === "stale";
    return {
      name: column.name,
      caption,
      stale,
      spoken: `${column.name}, ${caption}${stale ? ", stale" : ""}`,
    };
  });
}

/** The words a suite's path writes in lower case that a name for people spells with other capitals. */
const SPELLED: Readonly<Record<string, string>> = {
  typescript: "TypeScript",
  javascript: "JavaScript",
};

/** Returns a suite's name for people from the rest of its path, such as "TypeScript standard audit" from "typescript-standard". */
export function suiteName(rest: string): string {
  const words = rest.split("-").filter((word) => word !== "");
  const first = words[0] ?? rest;
  const spelled = SPELLED[first] ?? `${first.charAt(0).toUpperCase()}${first.slice(1)}`;
  return [spelled, ...words.slice(1), "audit"].join(" ");
}

/** Returns the matrix cell of `cell`, in `row`, in `column`. */
function matrixCell(name: string, row: Row, cell: Cell, column: RepositoryColumn): MatrixCell {
  const caption = cellCaption(row, cell, column);
  const spoken = `${column.name}, ${name}: ${spokenState(cell.state, cell.code)}`;
  return {
    repository: column.name,
    state: cell.state,
    word: cellWord(cell.state, cell.code),
    caption,
    spoken: caption === null ? spoken : `${spoken}, ${caption}`,
  };
}

/** Returns the matrix row of a rule. */
function ruleRow(document: ComplianceDocument, row: Row): MatrixRow {
  const name = rowName(row);
  return {
    key: `${row.id} ${row.source ?? "suite"}`,
    id: row.id,
    section: row.section,
    name,
    checker: checkerWord(row.checker, false),
    rows: [row],
    cells: document.repositories.map((column, index) => {
      const cell = row.cells[index] ?? absent(column.name);
      return matrixCell(name, row, cell, column);
    }),
  };
}

/** The cell of a row that does not reach `repository`. */
function absent(repository: string): Cell {
  return {
    repository,
    state: "absent",
    code: null,
    result: null,
    facts: null,
    run: null,
    age_s: null,
  };
}

/** Returns the part of a suite's path after a repository's short name, such as "rust-standard". */
function suitePattern(id: string, shortNames: readonly string[]): string | null {
  const name = id.startsWith("/suites/") ? id.slice("/suites/".length) : null;
  if (name === null) return null;
  // The longest name first, so `api-gateway-review` is api-gateway's and not api's.
  const owner = [...shortNames]
    .sort((a, b) => b.length - a.length)
    .find((short) => name.startsWith(`${short}-`));
  return owner === undefined ? null : name.slice(owner.length + 1);
}

/**
 * Returns the suites' rows. A suite named for a repository, such as
 * /suites/api-rust-standard, joins the other repositories' suites of the
 * same pattern in one row, and each column takes its own repository's
 * suite. A suite named for no repository keeps a row of its own.
 */
function suiteRows(document: ComplianceDocument, rows: readonly Row[]): MatrixRow[] {
  const shortNames = document.repositories.map((column) => shortName(column.name));
  const patterns = new Map<string, Row[]>();
  const result: MatrixRow[] = [];
  for (const row of rows) {
    const pattern = suitePattern(row.id, shortNames);
    if (pattern === null) {
      result.push({ ...ruleRow(document, row), name: shortName(row.id), checker: row.id });
      continue;
    }
    const members = patterns.get(pattern);
    if (members === undefined) patterns.set(pattern, [row]);
    else members.push(row);
  }
  for (const [pattern, members] of patterns) {
    const first = members[0];
    if (first === undefined) continue;
    const name = suiteName(pattern);
    const cells = document.repositories.map((column, index) => {
      const own = members.find((row) => row.id === `/suites/${shortName(column.name)}-${pattern}`);
      const reaching = members.find((row) => row.cells[index]?.state !== "absent");
      const row = own ?? reaching ?? first;
      return matrixCell(name, row, row.cells[index] ?? absent(column.name), column);
    });
    result.push({
      key: `/suites/<repo>-${pattern}`,
      id: `/suites/<repo>-${pattern}`,
      section: "suite",
      name,
      checker: `<repo>-${pattern}`,
      rows: members,
      cells,
    });
  }
  return result;
}

/** Returns the rank of a matrix row's worst cell, with an absent cell ranked last. */
function worstRank(row: MatrixRow): number {
  const ranks = row.cells.map((cell) => CELL_STATES_WORST_FIRST.indexOf(cell.state));
  return Math.min(CELL_STATES_WORST_FIRST.length, ...ranks.filter((rank) => rank >= 0));
}

/**
 * Returns every row of the matrix, in the document's order of sections. The
 * measured rules list the worst first, as artboard 54 lists its failing
 * rules above its stale ones and those above the current ones, and keep the
 * document's order between rows of one state.
 */
export function matrixRows(document: ComplianceDocument): MatrixRow[] {
  const suites = document.rows.filter((row) => row.section === "suite");
  const rules = (section: Section) =>
    document.rows.filter((row) => row.section === section).map((row) => ruleRow(document, row));
  const measured = rules("measured");
  const ranks = new Map(measured.map((row) => [row.key, worstRank(row)]));
  return [
    ...measured.sort((a, b) => (ranks.get(a.key) ?? 0) - (ranks.get(b.key) ?? 0)),
    ...rules("zero"),
    ...suiteRows(document, suites),
    ...rules("written"),
  ];
}
