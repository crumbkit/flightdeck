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
 * Returns true when the form shows the argument: the tool's own arguments
 * and crumbkit's `--limit` and `--all`.
 *
 * A count argument, such as `-v -v`, is left out, because the run request
 * has no value for a count. It needs a count value in the request first.
 */
function shown(argument: Argument): boolean {
  if (argument.arity.kind === "count") {
    return false;
  }
  return argument.crumbkit_flag === null || pagingFlags.includes(argument.crumbkit_flag);
}

/** Returns the form of `command`. */
export function formOf(command: Command): Form {
  const candidates = command.arguments.filter(shown);
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

/** Returns the run request of `draft`: the chosen argument of each field that has a value. */
export function requestOf(command: Command, draft: Draft): RunRequest {
  const form = formOf(command);
  const values: Record<string, Value> = {};
  for (const group of [...form.required, ...form.more]) {
    const chosen = chosenOf(draft, group);
    const value = chosen === undefined ? undefined : draft.values[chosen.name];
    if (chosen !== undefined && value !== undefined && !isEmpty(value)) {
      values[chosen.name] = value;
    }
  }
  return { path: command.path, values };
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

/**
 * Returns the command line of `request` as a person would type it: the
 * tool, the path, the positional values, then each option in the order the
 * command declares it. The server's own line replaces it once a run starts.
 */
export function lineOf(tool: string, command: Command, request: RunRequest): string {
  const words = [tool, ...command.path];
  const positionals = command.arguments.filter((each) => each.kind === "positional");
  const named = command.arguments.filter((each) => each.kind === "named");
  for (const argument of [...positionals, ...named]) {
    const value = request.values[argument.name];
    if (value === undefined) {
      continue;
    }
    const flag = argument.kind === "named" ? [argument.name] : [];
    if (value === true) {
      words.push(...flag);
    } else if (typeof value === "string") {
      words.push(...flag, quote(value));
    } else {
      for (const item of value) {
        words.push(...flag, quote(item));
      }
    }
  }
  return words.join(" ");
}
