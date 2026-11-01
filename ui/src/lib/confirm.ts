import type { Command } from "@/api/model";
import { resultOf } from "@/api/result";
import type { RunEvent } from "@/api/runs";

/**
 * Returns the number of rows that a preview's lines would change: the rows
 * of a listing, one for a record, or null when the result has no rows.
 */
export function countOf(lines: string[]): number | null {
  const { result } = resultOf(lines);
  switch (result.kind) {
    case "table":
      return result.rows.length;
    case "record":
      return 1;
    default:
      return null;
  }
}

/** Returns `word` with its first letter in capitals, such as Remove. */
function capitalised(word: string): string {
  return word.charAt(0).toUpperCase() + word.slice(1);
}

/**
 * Returns the resource of a command's path for `count` rows, such as book
 * for one row of `books remove`, or item for a path that is only its verb.
 *
 * This is a shortcut. It makes a regular English plural singular by
 * dropping its last `s`, or `ies` for `y`, and misses irregular plurals,
 * such as `people`. A singular name in the tool's own model replaces it
 * when crumbkit declares one.
 */
function resourceOf(command: Command, count: number): string {
  const plural = command.path.at(-2) ?? "items";
  if (count !== 1) {
    return plural;
  }
  if (plural.endsWith("ies")) {
    return `${plural.slice(0, -3)}y`;
  }
  return plural.endsWith("s") ? plural.slice(0, -1) : plural;
}

/** Returns the command's verb, the last word of its path, such as Remove. */
function verbOf(command: Command): string {
  return capitalised(command.path.at(-1) ?? "run");
}

/**
 * Returns the confirm button's words: the verb and the count, such as
 * "Remove 1 book", or the verb alone when the preview counted nothing.
 */
export function confirmLabel(command: Command, count: number | null): string {
  const verb = verbOf(command);
  return count === null ? verb : `${verb} ${count} ${resourceOf(command, count)}`;
}

/**
 * Returns the question of the confirmation, such as "Remove 1 book?", or
 * one that names the exact line when the preview counted nothing.
 */
export function question(command: Command, count: number | null, line: string): string {
  return count === null ? `Run ${line}?` : `${confirmLabel(command, count)}?`;
}

/** Returns what the command changes in words, such as "Destructive. Idempotent." */
export function effectsInWords(command: Command): string {
  const { effects } = command;
  if (effects === null) {
    return "Effects unknown, so flightdeck treats this command as destructive.";
  }
  const kind = effects.destructive ? "Destructive." : "Additive.";
  return `${kind} ${effects.idempotent ? "Idempotent." : "Not idempotent."}`;
}

/**
 * Returns the line that the confirm button runs: the preview's line with
 * `--yes` when the command takes it, which the server adds.
 */
export function confirmedLine(command: Command, line: string): string {
  const takesYes = command.arguments.some((each) => each.crumbkit_flag === "--yes");
  return takesYes ? `${line} --yes` : line;
}

/**
 * Returns true when a run ended with crumbkit's failure for a command that
 * asked for confirmation, such as `shelf::confirmation::needs_yes`.
 */
export function asksForYes(event: RunEvent): boolean {
  return (
    event.event === "ended" &&
    event.outcome.kind === "tool_failure" &&
    event.outcome.failure.code.endsWith("::confirmation::needs_yes")
  );
}
