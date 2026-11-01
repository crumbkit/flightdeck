import type { Argument, Command, Tool } from "@/api/model";
import type { RunRequest, Value } from "@/api/runs";

/**
 * Splits a line into words at spaces, keeping a single- or double-quoted
 * part as one word without its quotes.
 *
 * This reads the command lines of a tool's own examples, which use plain
 * words and simple quotes. It handles no backslash escapes and no quote that
 * is left open; the composer's command line editor replaces it with a full
 * POSIX tokenizer.
 */
export function words(line: string): string[] {
  return [...line.matchAll(/'([^']*)'|"([^"]*)"|(\S+)/g)].map(
    (match) => match[1] ?? match[2] ?? match[3] ?? "",
  );
}

/** Returns the runnable command whose path the words start with, longest first. */
function commandAt(tool: Tool, rest: string[]): Command | undefined {
  return [...tool.commands]
    .sort((a, b) => b.path.length - a.path.length)
    .find((command) => command.path.every((word, index) => rest[index] === word));
}

/** crumbkit's flags that flightdeck sets itself, which an example may carry. */
const setByFlightdeck = ["--json", "--full", "--no-input", "--long", "--dry-run", "--yes"];

/** Adds `item` to the many value of `name`, or sets the one value. */
function add(values: Record<string, Value>, argument: Argument, item: string): void {
  const current = values[argument.name];
  values[argument.name] =
    argument.arity.kind === "many" ? [...(Array.isArray(current) ? current : []), item] : item;
}

/** Returns true for a placeholder of a call, such as `<ID>...` or `<N>`. */
export function isPlaceholder(word: string): boolean {
  return /^<[^<>]+>(\.\.\.)?$/.test(word);
}

/**
 * Returns the run request of an example's command line, such as
 * `shelf books show 2 5`, or null when the line does not start with the
 * tool's name and one of its commands, or holds an argument the command
 * does not take. crumbkit's flags that flightdeck sets itself are dropped,
 * and `--fields` with its value, which only chooses columns. A placeholder,
 * such as the `<ID>...` of a next step's call, leaves its field empty.
 */
export function requestOfExample(tool: Tool, line: string): RunRequest | null {
  return requestOfWords(tool, words(line));
}

/** Returns the run request of a command line that is already split into words. */
export function requestOfWords(tool: Tool, line: string[]): RunRequest | null {
  const [name, ...rest] = line;
  const command = name === tool.name ? commandAt(tool, rest) : undefined;
  if (command === undefined) {
    return null;
  }
  const tokens = rest.slice(command.path.length);
  const positionals = command.arguments.filter((each) => each.kind === "positional");
  const values: Record<string, Value> = {};
  let position = 0;
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index] ?? "";
    if (!token.startsWith("-")) {
      const argument = positionals[position];
      if (argument === undefined) {
        return null;
      }
      if (!isPlaceholder(token)) {
        add(values, argument, token);
      }
      if (argument.arity.kind !== "many") {
        position += 1;
      }
      continue;
    }
    const [flag = "", inline] = token.split(/=(.*)/s);
    if (setByFlightdeck.includes(flag)) {
      continue;
    }
    const argument = command.arguments.find((each) => each.name === flag || each.short === flag);
    if (argument === undefined) {
      return null;
    }
    const takesValue = argument.arity.kind === "one" || argument.arity.kind === "many";
    const value = takesValue ? (inline ?? tokens[++index]) : undefined;
    if (argument.name === "--fields" || (value !== undefined && isPlaceholder(value))) {
      continue;
    }
    if (!takesValue) {
      values[argument.name] = true;
    } else if (value === undefined) {
      return null;
    } else {
      add(values, argument, value);
    }
  }
  return { path: command.path, values };
}
