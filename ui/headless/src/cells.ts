// What one cell says beyond its state: the caption under its chip, the
// sentence a list of failures gives it, and what a repository's page shows
// as measured. Each reads only the cell, its row and its column.
import { counted } from "@crumbkit/ui/a11y";
import type { Cell, RepositoryColumn, Row, RuleFacts, SuiteRun } from "./document.ts";
import {
  ageWord,
  budgetUnit,
  checkerField,
  checkerNumber,
  checkKind,
  daysWord,
  numberWord,
  shortCommit,
  underADay,
} from "./words.ts";

/** Returns how many files a budget found over its limit, with those the report left out. */
function overCount(facts: RuleFacts): number {
  return facts.over.length + facts.omitted.over;
}

/** Whether a budget counts each file on its own. */
function perFile(row: Row): boolean {
  return checkerField(row.checker, "per") !== null;
}

/** Returns a zero rule's measures as words, such as "remaining 2 and unknown 0". */
function measuresWord(facts: RuleFacts): string {
  const measures = Object.entries(facts.zero?.measures ?? {}).map(
    ([name, value]) => `${name} ${value === null ? "none" : String(value)}`,
  );
  if (measures.length === 0) return "no measure";
  return measures.length === 1
    ? (measures[0] ?? "")
    : `${measures.slice(0, -1).join(", ")} and ${measures.at(-1) ?? ""}`;
}

/** Returns the count a zero rule's first measure holds, or null. */
function remaining(facts: RuleFacts): number | null {
  const measures = facts.zero?.measures ?? {};
  const value = measures.remaining ?? Object.values(measures)[0] ?? null;
  return typeof value === "number" ? value : null;
}

/** Returns a suite run's caption: its age, or "today", and its commits since, such as "2 d · +6". */
export function runCaption(run: SuiteRun | null, ageS: number | null): string {
  if (run === null) return "no run";
  let age = "no time";
  if (ageS !== null) age = underADay(ageS) ? "today" : ageWord(ageS);
  const since = run.commits_since;
  return since === null || since === 0 ? age : `${age} · +${since}`;
}

/** Returns the caption under a cell's chip in the matrix, or null when the chip says it all. */
export function cellCaption(row: Row, cell: Cell, column: RepositoryColumn): string | null {
  if (row.section === "suite") {
    return cell.state === "absent" || cell.state === "not_readable"
      ? null
      : runCaption(cell.run, cell.age_s);
  }
  const facts = cell.facts;
  if (facts === null) return null;
  switch (cell.state) {
    case "failing":
      switch (cell.result) {
        case "over":
          return perFile(row) || facts.over.length > 0
            ? `${overCount(facts)} over`
            : numberWord(facts.used ?? 0);
        case "behind":
          return facts.behind[0]?.found ?? "behind";
        case "not_zero": {
          const left = remaining(facts);
          return left === null ? "not 0" : `${numberWord(left)} left`;
        }
        case "missing_measure":
          return "no measure";
        default:
          return null;
      }
    case "stale":
      if (row.section === "zero") {
        const age = facts.zero?.age_s ?? null;
        return age === null ? "no run" : ageWord(age);
      }
      return column.age_s === null ? null : ageWord(column.age_s);
    case "not_readable":
    case "current":
    case "written":
    case "absent":
      return null;
  }
}

/** Returns the sentence that says why a failing cell fails, or null for any other cell. */
export function failureDetail(row: Row, cell: Cell): string | null {
  if (cell.state !== "failing") return null;
  if (row.section === "suite") {
    const run = cell.run;
    if (run === null) return null;
    const when = cell.age_s === null ? "" : `, ${daysWord(cell.age_s)} ago`;
    const at = run.commit === null ? "" : ` at ${shortCommit(run.commit)}`;
    const cases = run.failed.map((each) => each.case);
    const which = cases.length === 0 ? "" : `: ${cases.join(", ")}`;
    return `The run${at}${when}, failed ${counted(run.failed.length, "case")}${which}.`;
  }
  const facts = cell.facts;
  if (facts === null) return null;
  switch (cell.result) {
    case "over": {
      const unit = checkerField(row.checker, "unit") ?? "units";
      const max = facts.max ?? checkerNumber(row.checker, "max") ?? 0;
      const first = facts.over[0];
      if (overCount(facts) === 1 && first !== undefined) {
        return `${first.file} is ${numberWord(first.used)} ${unit}, and the limit is ${numberWord(max)}.`;
      }
      if (overCount(facts) > 1) {
        return `${overCount(facts)} files are over the limit of ${numberWord(max)} ${unit}.`;
      }
      return `The files hold ${numberWord(facts.used ?? 0)} ${unit}, and the limit is ${numberWord(max)}.`;
    }
    case "behind": {
      const pin = facts.behind[0];
      const atLeast = checkerField(row.checker, "at_least");
      const needs = atLeast === null ? "" : `, and the rule needs at least ${atLeast}`;
      return pin === undefined
        ? `A pin is older than the rule allows${needs}.`
        : `${pin.file} line ${pin.line} pins ${pin.found}${needs}.`;
    }
    case "not_zero": {
      const age = facts.zero?.age_s ?? null;
      const when = age === null ? "The newest run" : `The run ${ageWord(age)} ago`;
      return `${when} reports ${measuresWord(facts)}.`;
    }
    case "missing_measure":
      return `The newest run of ${facts.zero?.suite ?? "its suite"} records ${measuresWord(facts)}.`;
    default:
      return null;
  }
}

/** Returns the sentence that says why a suite or zero rule cell is stale, or null. */
export function staleDetail(row: Row, cell: Cell): string | null {
  if (cell.state !== "stale") return null;
  if (row.section === "suite") {
    const run = cell.run;
    if (run === null) return `The suite ${row.id} has no clean full run yet.`;
    const ago = cell.age_s === null ? "" : ` ${daysWord(cell.age_s)} ago`;
    const at = run.commit === null ? "" : ` at ${shortCommit(run.commit)}`;
    const since =
      run.commits_since === null ? "" : `, with ${counted(run.commits_since, "commit")} since`;
    return `Last clean run${ago}${at}${since}.`;
  }
  if (row.section === "zero") {
    const zero = cell.facts?.zero ?? null;
    if (zero === null || zero.age_s === null) {
      return `The suite ${zero?.suite ?? checkerField(row.checker, "suite") ?? ""} has no run yet.`;
    }
    const allows = checkerNumber(row.checker, "within_days");
    const window = allows === null ? "" : `, and the rule allows ${counted(allows, "day")}`;
    return `The newest run is ${daysWord(zero.age_s)} old${window}.`;
  }
  return null;
}

/** Returns the age a stale cell sorts by, oldest first, with a check that never ran oldest of all. */
export function staleAge(row: Row, cell: Cell, column: RepositoryColumn): number {
  if (row.section === "suite") return cell.age_s ?? Number.POSITIVE_INFINITY;
  if (row.section === "zero") return cell.facts?.zero?.age_s ?? Number.POSITIVE_INFINITY;
  return column.age_s ?? Number.POSITIVE_INFINITY;
}

/** Returns what a repository's page shows a rule measured, such as "1 file over: src/a.rs 57,310". */
export function measuredText(row: Row, cell: Cell): string {
  const facts = cell.facts;
  if (cell.result === "unmeasured") return "not measured";
  if (cell.result === "unreadable") return `not readable: ${cell.code ?? "unknown"}`;
  if (facts === null) return "–";
  switch (checkKind(row.checker)) {
    case "budget": {
      const unit = budgetUnit(row.checker).split(" ")[0] ?? "units";
      if (facts.over.length > 0 || cell.result === "over") {
        if (!perFile(row)) return `${numberWord(facts.used ?? 0)} ${unit}`;
        const shown = facts.over.map((each) => `${each.file} ${numberWord(each.used)}`).join(", ");
        return `${counted(overCount(facts), "file")} over: ${shown}`;
      }
      const files = facts.files.length + facts.omitted.files;
      return perFile(row)
        ? `largest file ${numberWord(facts.used ?? 0)} ${unit}`
        : `${numberWord(facts.used ?? 0)} ${unit} in ${counted(files, "file")}`;
    }
    case "version": {
      const pin = facts.behind[0];
      if (pin !== undefined) return `${pin.file} pins ${pin.found}`;
      return facts.files.length === 0 ? "no pin found" : `reads ${facts.files.join(", ")}`;
    }
    case "zero": {
      const zero = facts.zero;
      if (zero === null || zero.age_s === null) return "no run yet";
      const allows = checkerNumber(row.checker, "within_days");
      const window = allows === null ? "" : `, and the rule allows ${allows} d`;
      return `${measuresWord(facts)} · ${ageWord(zero.age_s)} old${window}`;
    }
    default:
      return cell.result ?? "–";
  }
}
