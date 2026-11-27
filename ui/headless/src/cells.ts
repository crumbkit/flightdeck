// What one cell says beyond its state: the caption under its chip, the
// sentence a list of failures gives it, and what a repository's page shows
// as measured. Each reads only the cell, its row and its column. A sentence
// is a phrase whose paths, commits and measures the pages set in the mono
// face, and each has a plain string form for a screen reader.
import { counted } from "@crumbkit/ui/a11y";
import type { Cell, RepositoryColumn, Row, RuleFacts, SuiteRun } from "./document.ts";
import {
  ageWord,
  andPhrase,
  budgetUnit,
  checkerField,
  checkerNumber,
  checkKind,
  daysWord,
  mono,
  numberWord,
  type Phrase,
  phrase,
  phraseText,
  shortCommit,
  underADay,
} from "./words.ts";

/** What a zero rule's run measured, by measure. */
export type Measures = NonNullable<RuleFacts["zero"]>["measures"];

/** Returns how many files a budget found over its limit, with those the report left out. */
function overCount(facts: RuleFacts): number {
  return facts.over.length + facts.omitted.over;
}

/**
 * Returns a file as the pages name it, such as "AGENTS.md" for "/AGENTS.md":
 * a report gives paths from the repository's root, and the artboards drop
 * the leading slash.
 */
export function fileName(path: string): string {
  return path.replace(/^\//, "");
}

/**
 * Returns the files a report lists, as a sentence names them, with how many
 * there are counting those the build left out. Past three files, or when the
 * build left some out, it names the first two and counts the rest, such as
 * "AGENTS.md, CONTEXT.md and 48 more files".
 */
function filesPhrase(facts: RuleFacts): { readonly text: Phrase; readonly count: number } {
  const names = facts.files.map((file) => [mono(fileName(file))]);
  const count = names.length + facts.omitted.files;
  if (count <= 3 && facts.omitted.files === 0) return { text: andPhrase(names), count };
  if (names.length === 0) return { text: [counted(count, "file")], count };
  const shown = names.slice(0, 2);
  const listed = shown.flatMap((name, at) => (at === 0 ? [name] : [", ", name]));
  return { text: phrase(...listed, ` and ${count - shown.length} more files`), count };
}

/** Whether a budget counts each file on its own. */
function perFile(row: Row): boolean {
  return checkerField(row.checker, "per") !== null;
}

/** Returns a zero rule's measures as words, such as "remaining 2 and unknown 0", each in the mono face. */
function measuresPhrase(measures: Measures): Phrase {
  const each = Object.entries(measures).map(([name, value]) => [
    mono(`${name} ${value === null ? "none" : String(value)}`),
  ]);
  return each.length === 0 ? ["no measure"] : andPhrase(each);
}

/** Returns the sentence of a zero rule whose newest run measured `measures`, not all 0. */
export function notZeroPhrase(measures: Measures): Phrase {
  return phrase("The newest run reports ", measuresPhrase(measures), ".");
}

/** Returns the count a zero rule's first measure holds, or null. */
function remaining(facts: RuleFacts): number | null {
  const measures = facts.zero?.measures ?? {};
  const value = measures.remaining ?? Object.values(measures)[0] ?? null;
  return typeof value === "number" ? value : null;
}

/** Returns a suite run's caption: its age, or "today", and its commits since, such as "2 d · +6". */
function runCaption(run: SuiteRun | null, ageS: number | null): string {
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

/** Returns the sentence that says why a failing cell fails, with its paths, commits and measures in the mono face, or null for any other cell. */
export function failurePhrase(row: Row, cell: Cell): Phrase | null {
  if (cell.state !== "failing") return null;
  if (row.section === "suite") {
    const run = cell.run;
    if (run === null) return null;
    const when = cell.age_s === null ? "" : `, ${daysWord(cell.age_s)} ago`;
    const at = run.commit === null ? [] : [" at ", mono(shortCommit(run.commit))];
    const cases = run.failed.map((each) => each.case);
    const which = cases.length === 0 ? "" : `: ${cases.join(", ")}`;
    return phrase("The run", at, `${when}, failed ${counted(run.failed.length, "case")}${which}.`);
  }
  const facts = cell.facts;
  if (facts === null) return null;
  switch (cell.result) {
    case "over": {
      const unit = checkerField(row.checker, "unit") ?? "units";
      const max = numberWord(facts.max ?? checkerNumber(row.checker, "max") ?? 0);
      const first = facts.over[0];
      if (overCount(facts) === 1 && first !== undefined) {
        return phrase(
          mono(fileName(first.file)),
          ` is ${numberWord(first.used)} ${unit}, and the limit is ${max}.`,
        );
      }
      if (overCount(facts) > 1) {
        return [`${overCount(facts)} files are over the limit of ${max} ${unit}.`];
      }
      const used = `${numberWord(facts.used ?? 0)} ${unit}`;
      const files = filesPhrase(facts);
      if (files.count === 0) return [`The files hold ${used}, and the limit is ${max}.`];
      if (files.count === 1) return phrase(files.text, ` holds ${used}, and the limit is ${max}.`);
      return phrase(files.text, ` hold ${used} together.`);
    }
    case "behind": {
      // The rule's name says its minimum, as artboard 54's Node item does, so
      // the sentence says only what the file pins.
      const pin = facts.behind[0];
      if (pin !== undefined) {
        return phrase(mono(fileName(pin.file)), ` line ${pin.line} pins ${pin.found}.`);
      }
      const atLeast = checkerField(row.checker, "at_least");
      const needs = atLeast === null ? "" : `, and the rule needs at least ${atLeast}`;
      return [`A pin is older than the rule allows${needs}.`];
    }
    case "not_zero":
      return notZeroPhrase(facts.zero?.measures ?? {});
    case "missing_measure":
      return phrase(
        "The newest run of ",
        mono(facts.zero?.suite ?? "its suite"),
        " records ",
        measuresPhrase(facts.zero?.measures ?? {}),
        ".",
      );
    default:
      return null;
  }
}

/** Returns the sentence that says why a failing cell fails, or null for any other cell. */
export function failureDetail(row: Row, cell: Cell): string | null {
  const said = failurePhrase(row, cell);
  return said === null ? null : phraseText(said);
}

/** Returns the sentence that says why a suite or zero rule cell is stale, with its paths and commits in the mono face, or null. */
export function stalePhrase(row: Row, cell: Cell): Phrase | null {
  if (cell.state !== "stale") return null;
  if (row.section === "suite") {
    const run = cell.run;
    if (run === null) return phrase("The suite ", mono(row.id), " has no clean full run yet.");
    const ago = cell.age_s === null ? "" : ` ${daysWord(cell.age_s)} ago`;
    const at = run.commit === null ? [] : [" at ", mono(shortCommit(run.commit))];
    const since =
      run.commits_since === null ? "" : `, with ${counted(run.commits_since, "commit")} since`;
    return phrase(`Last clean run${ago}`, at, `${since}.`);
  }
  if (row.section === "zero") {
    const zero = cell.facts?.zero ?? null;
    if (zero === null || zero.age_s === null) {
      const suite = zero?.suite ?? checkerField(row.checker, "suite") ?? "";
      return phrase("The suite ", mono(suite), " has no run yet.");
    }
    const allows = checkerNumber(row.checker, "within_days");
    const window = allows === null ? "" : `, and the rule allows ${counted(allows, "day")}`;
    return [`The newest run is ${daysWord(zero.age_s)} old${window}.`];
  }
  return null;
}

/** Returns the age a stale cell sorts by, oldest first, with a check that never ran oldest of all. */
export function staleAge(row: Row, cell: Cell, column: RepositoryColumn): number {
  if (row.section === "suite") return cell.age_s ?? Number.POSITIVE_INFINITY;
  if (row.section === "zero") return cell.facts?.zero?.age_s ?? Number.POSITIVE_INFINITY;
  return column.age_s ?? Number.POSITIVE_INFINITY;
}

/** Returns what a repository's page shows a rule measured, with its paths and measures in the mono face. */
export function measuredPhrase(row: Row, cell: Cell): Phrase {
  const facts = cell.facts;
  if (cell.result === "unmeasured") return ["not measured"];
  if (cell.result === "unreadable") return phrase("not readable: ", mono(cell.code ?? "unknown"));
  if (facts === null) return ["–"];
  switch (checkKind(row.checker)) {
    case "budget": {
      const unit = budgetUnit(row.checker).split(" ")[0] ?? "units";
      if (facts.over.length > 0 || cell.result === "over") {
        if (!perFile(row)) return [`${numberWord(facts.used ?? 0)} ${unit}`];
        const shown = facts.over.flatMap((each, at) => [
          at === 0 ? "" : ", ",
          mono(fileName(each.file)),
          ` ${numberWord(each.used)}`,
        ]);
        return phrase(`${counted(overCount(facts), "file")} over: `, ...shown);
      }
      const files = facts.files.length + facts.omitted.files;
      return perFile(row)
        ? [`largest file ${numberWord(facts.used ?? 0)} ${unit}`]
        : [`${numberWord(facts.used ?? 0)} ${unit} in ${counted(files, "file")}`];
    }
    case "version": {
      const pin = facts.behind[0];
      if (pin !== undefined) return phrase(mono(fileName(pin.file)), ` pins ${pin.found}`);
      // A report gives the version found only for a pin behind the minimum,
      // so a pin within it reads as at least that minimum, not its version.
      const atLeast = checkerField(row.checker, "at_least");
      const files = filesPhrase(facts);
      if (files.count === 0) return ["no pin found"];
      return atLeast === null
        ? phrase("reads ", files.text)
        : phrase(files.text, ` ${files.count === 1 ? "pins" : "pin"} at least ${atLeast}`);
    }
    case "zero": {
      const zero = facts.zero;
      if (zero === null || zero.age_s === null) return ["no run yet"];
      const allows = checkerNumber(row.checker, "within_days");
      const window = allows === null ? "" : `, and the rule allows ${allows} d`;
      return phrase(measuresPhrase(zero.measures), ` · ${ageWord(zero.age_s)} old${window}`);
    }
    default:
      return [cell.result ?? "–"];
  }
}

/** Returns what a repository's page shows a rule measured, such as "1 file over: src/a.rs 57,310". */
export function measuredText(row: Row, cell: Cell): string {
  return phraseText(measuredPhrase(row, cell));
}
