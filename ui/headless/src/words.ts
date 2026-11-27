// The words the compliance pages show: each state's word, ages, numbers,
// times and checkers. Every page module builds its text from these, so one
// state reads the same on every page.
import type { CellState, Checker, ReportState } from "./document.ts";

/** The code of a column whose repository has no report file. */
export const NO_REPORT = "flightdeck::compliance::no_report";

/** The code of a column whose report file is not a report the build reads. */
export const INVALID_REPORT = "flightdeck::compliance::invalid_report";

const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Returns an age in the largest whole unit, such as "4 min", "3 h" or "9 d". */
export function ageWord(seconds: number): string {
  if (seconds >= DAY) return `${Math.floor(seconds / DAY)} d`;
  if (seconds >= HOUR) return `${Math.floor(seconds / HOUR)} h`;
  if (seconds >= MINUTE) return `${Math.floor(seconds / MINUTE)} min`;
  return "under 1 min";
}

/** Returns whole days in words, such as "1 day" or "9 days". */
export function daysWord(seconds: number): string {
  const days = Math.floor(seconds / DAY);
  return days === 1 ? "1 day" : `${days} days`;
}

/** Whether `seconds` is under a day. */
export function underADay(seconds: number): boolean {
  return seconds < DAY;
}

const numbers = new Intl.NumberFormat("en-US");

/** Returns a whole number with its thousands grouped, such as "1,184". */
export function numberWord(value: number): string {
  return numbers.format(value);
}

const MONTHS = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** Returns an RFC 3339 time as its hour and day in UTC, such as "06:00 UTC on 25 November 2026". */
export function moment(iso: string): string {
  const at = new Date(iso);
  const hours = String(at.getUTCHours()).padStart(2, "0");
  const minutes = String(at.getUTCMinutes()).padStart(2, "0");
  const month = MONTHS[at.getUTCMonth()] ?? "";
  return `${hours}:${minutes} UTC on ${at.getUTCDate()} ${month} ${at.getUTCFullYear()}`;
}

/** Returns an RFC 3339 time's day and month in UTC, such as "16 November". */
export function dayWord(iso: string): string {
  const at = new Date(iso);
  return `${at.getUTCDate()} ${MONTHS[at.getUTCMonth()] ?? ""}`;
}

/**
 * Returns how often a job runs, from its interval in hours: "every hour",
 * "every 6 hours", "daily", "every 2 days", "weekly", or "every 30 hours"
 * for an interval that is not whole days.
 */
export function cadenceWord(hours: number): string {
  if (hours === 1) return "every hour";
  if (hours === 24) return "daily";
  if (hours === 168) return "weekly";
  if (hours % 24 === 0) return `every ${hours / 24} days`;
  return `every ${hours} hours`;
}

/** Returns an RFC 3339 time in UTC as a short line names it, such as "25 Nov 2026, 06:00 UTC". */
export function shortMoment(iso: string): string {
  const at = new Date(iso);
  const hours = String(at.getUTCHours()).padStart(2, "0");
  const minutes = String(at.getUTCMinutes()).padStart(2, "0");
  const month = (MONTHS[at.getUTCMonth()] ?? "").slice(0, 3);
  return `${at.getUTCDate()} ${month} ${at.getUTCFullYear()}, ${hours}:${minutes} UTC`;
}

/** Returns a commit's first seven characters, as Git abbreviates it. */
export function shortCommit(sha: string): string {
  return sha.slice(0, 7);
}

/** Returns the last segment of a path or a repository's name, such as "file-size" or "app". */
export function shortName(path: string): string {
  const parts = path.split("/").filter((part) => part !== "");
  return parts.at(-1) ?? path;
}

/** Returns a rule's name for people: its requirement without the full stop, or its path. */
export function rowName(row: { readonly id: string; readonly requirement: string | null }): string {
  if (row.requirement === null || row.requirement.trim() === "") return row.id;
  return row.requirement.trim().replace(/\.$/, "");
}

/** Returns the word a cell's chip shows. A column with no report says so in each of its cells. */
export function cellWord(state: CellState, code: string | null): string {
  switch (state) {
    case "failing":
      return "failing";
    case "stale":
      return "stale";
    case "not_readable":
      if (code === NO_REPORT) return "no report";
      if (code === INVALID_REPORT) return "invalid report";
      return "not readable";
    case "current":
      return "current";
    case "written":
      return "written";
    case "absent":
      return "–";
  }
}

/** Returns the word a screen reader speaks for a cell's state, which for an absent cell is not a dash. */
export function spokenState(state: CellState, code: string | null): string {
  return state === "absent" ? "does not apply" : cellWord(state, code);
}

/** Returns the word a repository's report state shows. */
export function reportWord(report: ReportState): string {
  switch (report) {
    case "current":
      return "current";
    case "stale":
      return "stale";
    case "no_report":
      return "no report";
    case "invalid_report":
      return "invalid report";
  }
}

/** Returns a checker's field as text, or null when it is missing or not text or a number. */
export function checkerField(checker: Checker | null, field: string): string | null {
  const value = checker?.[field];
  if (typeof value === "string") return value;
  if (typeof value === "number") return String(value);
  return null;
}

/** Returns a checker's numeric field, or null. */
export function checkerNumber(checker: Checker | null, field: string): number | null {
  const value = checker?.[field];
  return typeof value === "number" ? value : null;
}

/** Returns the kind of check a checker runs, or "written" for a rule with none. */
export function checkKind(checker: Checker | null): string {
  return checkerField(checker, "check") ?? "written";
}

/** Returns a budget's unit, such as "words" or "bytes per file". */
export function budgetUnit(checker: Checker | null): string {
  const unit = checkerField(checker, "unit") ?? "units";
  const per = checkerField(checker, "per");
  return per === null ? unit : `${unit} per ${per}`;
}

/**
 * Returns what a checker checks, such as "budget · words". The long form,
 * which the Rules page and a repository's page show, adds the limit, such as
 * "budget · words · max 1,000".
 */
export function checkerWord(checker: Checker | null, long: boolean): string {
  const kind = checkKind(checker);
  switch (kind) {
    case "budget": {
      const max = checkerNumber(checker, "max");
      const base = `budget · ${budgetUnit(checker)}`;
      return long && max !== null ? `${base} · max ${numberWord(max)}` : base;
    }
    case "version": {
      const atLeast = checkerField(checker, "at_least");
      return long && atLeast !== null ? `version · at least ${atLeast}` : "version";
    }
    case "zero": {
      const days = checkerNumber(checker, "within_days");
      return days === null ? "zero" : `zero · within ${days} d`;
    }
    case "judged": {
      const rubric = checkerField(checker, "rubric");
      return long && rubric !== null ? `judged · ${rubric}` : "judged";
    }
    case "pattern": {
      const quote = checkerField(checker, "quote");
      return long && quote !== null ? `pattern · ${quote}` : "pattern";
    }
    case "written":
      return kind;
    default:
      return `unknown checker ${kind}`;
  }
}

/**
 * A piece of a sentence: plain words, or a path, commit or measure that the
 * pages set in the mono face, as the artboards do inside notes.
 */
export type Piece = string | { readonly mono: string };

/** A sentence in pieces, which a page draws with its mono pieces in the mono face. */
export type Phrase = readonly Piece[];

/** Returns a piece the pages set in the mono face. */
export function mono(text: string): Piece {
  return { mono: text };
}

/** Returns pieces and phrases as one phrase, with neighbouring words joined and empty words dropped. */
export function phrase(...parts: readonly (Piece | Phrase)[]): Phrase {
  const out: Piece[] = [];
  const flat = parts.flatMap((part): readonly Piece[] =>
    typeof part === "string" || "mono" in part ? [part] : part,
  );
  for (const piece of flat) {
    const last = out.at(-1);
    if (piece === "") continue;
    if (typeof piece === "string" && typeof last === "string") out[out.length - 1] = last + piece;
    else out.push(piece);
  }
  return out;
}

/** Returns a phrase as one string, for a screen reader and for a page that draws it in one face. */
export function phraseText(phrase: Phrase): string {
  return phrase.map((piece) => (typeof piece === "string" ? piece : piece.mono)).join("");
}

/** Joins phrases as a sentence joins names: "a", "a and b", "a, b and c". */
export function andPhrase(phrases: readonly Phrase[]): Phrase {
  return phrase(
    ...phrases.flatMap((each, at) => {
      if (at === 0) return [each];
      return [at === phrases.length - 1 ? " and " : ", ", each];
    }),
  );
}

/** Joins names as a sentence does: "a", "a and b", "a, b and c". */
export function andList(names: readonly string[]): string {
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} and ${names.at(-1) ?? ""}`;
}
