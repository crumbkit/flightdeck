import type { Argument, Command, CommandPath } from "@/api/model";
import type { RunRequest, Value } from "@/api/runs";

/** crumbkit's own flags that the page offers, because they choose a page. */
const pagingFlags = ["--limit", "--all"];

/**
 * A field of the form: one argument, or an argument and the arguments of
 * its `one_of` that can stand in for it, between which a switch chooses.
 */
export type FieldGroup = { choices: Argument[] };

/** The composer's form: required fields first, then the more options. */
export type Form = { required: FieldGroup[]; more: FieldGroup[] };

/**
 * Returns true when the form of `command` shows the argument: the tool's
 * own arguments, crumbkit's `--limit` and `--all`, and `--long` when the
 * command's rows have long columns, which the result's table then shows.
 *
 * A count argument, such as `-v -v`, is left out, because the run request
 * has no value for a count. It needs a count value in the request first.
 */
export function shows(command: Command, argument: Argument): boolean {
  if (argument.arity.kind === "count") {
    return false;
  }
  if (argument.crumbkit_flag === "--long") {
    return command.result.columns.some((column) => column.long);
  }
  return argument.crumbkit_flag === null || pagingFlags.includes(argument.crumbkit_flag);
}

/** Returns the form of `command`. */
export function formOf(command: Command): Form {
  const candidates = command.arguments.filter((argument) => shows(command, argument));
  const placed = new Set<string>();
  const groups: FieldGroup[] = [];
  for (const argument of candidates) {
    if (placed.has(argument.name)) {
      continue;
    }
    const alternatives = argument.one_of.flatMap((name) => {
      const found = candidates.find((each) => each.name === name && !placed.has(name));
      return found === undefined ? [] : [found];
    });
    const choices = [argument, ...alternatives];
    for (const choice of choices) {
      placed.add(choice.name);
    }
    groups.push({ choices });
  }
  return {
    required: groups.filter((group) => group.choices[0]?.required === true),
    more: groups.filter((group) => group.choices[0]?.required !== true),
  };
}

/**
 * What the person has entered for one command: a value for each argument,
 * by its name in the model, and the chosen alternative of each switch, by
 * the name of the group's first argument.
 */
export type Draft = {
  path: CommandPath;
  values: Record<string, Value>;
  chosen: Record<string, string>;
};

/** Returns the empty draft of `command`. */
export function emptyDraft(command: Command): Draft {
  return { path: command.path, values: {}, chosen: {} };
}

/** Returns `draft` with `value` for the argument `name`, or without it for null. */
export function setValue(draft: Draft, name: string, value: Value | null): Draft {
  const { [name]: _old, ...values } = draft.values;
  return { ...draft, values: value === null ? values : { ...values, [name]: value } };
}

/** Returns the argument that a group's switch has chosen. */
export function chosenOf(draft: Draft, group: FieldGroup): Argument | undefined {
  const [first] = group.choices;
  const name = first === undefined ? undefined : draft.chosen[first.name];
  return group.choices.find((choice) => choice.name === name) ?? first;
}

/** Returns true when `value` sends nothing: a flag that is off or an empty value. */
function isEmpty(value: Value | undefined): boolean {
  if (value === undefined) {
    return true;
  }
  if (value === true) {
    return false;
  }
  return value.length === 0;
}

/** Returns the value of the chosen argument of each field that has one, by the argument's name. */
function fieldValues(command: Command, draft: Draft): Record<string, Value> {
  const form = formOf(command);
  const values: Record<string, Value> = {};
  for (const group of [...form.required, ...form.more]) {
    const chosen = chosenOf(draft, group);
    const value = chosen === undefined ? undefined : draft.values[chosen.name];
    if (chosen !== undefined && value !== undefined && !isEmpty(value)) {
      values[chosen.name] = value;
    }
  }
  return values;
}

/**
 * Returns the run request of `draft`: the chosen argument of each field
 * that has a value. `--long` stays out of it, because it only chooses the
 * columns of the result's table, which the page draws.
 */
export function requestOf(command: Command, draft: Draft): RunRequest {
  const { "--long": _long, ...values } = fieldValues(command, draft);
  return { path: command.path, values };
}

/** Returns true when `draft` asks for the long columns of the result's table. */
export function longOf(draft: Draft): boolean {
  return draft.values["--long"] === true;
}

/** Returns the command line of `draft`, with `--long` when it asks for the long columns. */
export function lineOfDraft(tool: string, command: Command, draft: Draft): string {
  return lineOf(tool, command, { path: command.path, values: fieldValues(command, draft) });
}

/** Returns the names of the required fields whose chosen argument has no value. */
export function missing(command: Command, draft: Draft): string[] {
  return formOf(command).required.flatMap((group) => {
    const chosen = chosenOf(draft, group);
    return chosen === undefined || isEmpty(draft.values[chosen.name]) ? [chosen?.name ?? ""] : [];
  });
}

/** Returns the draft of a run's request, with each switch on the alternative it used. */
export function draftFrom(command: Command, request: RunRequest): Draft {
  const chosen: Record<string, string> = {};
  const form = formOf(command);
  for (const group of [...form.required, ...form.more]) {
    const [first] = group.choices;
    const used = group.choices.find((choice) => request.values[choice.name] !== undefined);
    if (first !== undefined && used !== undefined) {
      chosen[first.name] = used.name;
    }
  }
  return { path: command.path, values: { ...request.values }, chosen };
}

/** Quotes a word for a POSIX shell when it holds anything but plain characters. */
function quote(word: string): string {
  return /^[\w./:=,@%+-]+$/.test(word) ? word : `'${word.replaceAll("'", "'\\''")}'`;
}

type CommandArgument = Command["arguments"][number];

/** Returns the values that `request` gives `argument`, as plain words. */
function valuesOf(argument: CommandArgument, request: RunRequest): string[] {
  const value = request.values[argument.name];
  if (value === undefined || value === true) {
    return [];
  }
  return typeof value === "string" ? [value] : value;
}

/**
 * Returns the positional values of `request`, quoted, and whether the line
 * writes `--` before them: when a value starts with a dash, or an argument
 * that takes its values after `--` has some, as the server's line has them.
 */
export function tailOf(
  command: Command,
  request: RunRequest,
): { words: string[]; separated: boolean } {
  const given = command.arguments
    .filter((each) => each.kind === "positional")
    .map((each) => ({ each, values: valuesOf(each, request) }));
  const words = given.flatMap(({ values }) => values.map(quote));
  const separated =
    words.some((word) => word.startsWith("-")) ||
    given.some(({ each, values }) => each.after_double_dash && values.length > 0);
  return { words, separated };
}

/**
 * Returns the program that `request` starts for a command that runs one:
 * the values of the argument that takes them after `--`, such as "cargo
 * run", or the command's own words when it has none.
 */
export function programOf(command: Command, request: RunRequest): string {
  const words = command.arguments
    .filter((each) => each.after_double_dash)
    .flatMap((each) => valuesOf(each, request));
  return (words.length > 0 ? words : command.path).join(" ");
}

/**
 * Returns the command line of `request` as a person would type it: the
 * tool, the path, the positional values, then each option in the order the
 * command declares it. When the values go after `--`, the options come
 * first and then `--` and the values, as the server's line has them. The
 * server's own line replaces it once a run starts.
 */
export function lineOf(tool: string, command: Command, request: RunRequest): string {
  const { words: positional, separated } = tailOf(command, request);
  const named = command.arguments
    .filter((each) => each.kind === "named")
    .flatMap((each) => {
      const value = request.values[each.name];
      if (value === undefined) {
        return [];
      }
      return value === true
        ? [each.name]
        : valuesOf(each, request).flatMap((item) => [each.name, quote(item)]);
    });
  const words = separated ? [...named, "--", ...positional] : [...positional, ...named];
  return [tool, ...command.path, ...words].join(" ");
}
