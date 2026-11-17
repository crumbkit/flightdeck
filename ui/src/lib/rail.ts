import type { Command, Tool } from "@/api/model";

/** A heading of the rail and the commands under it, or null for none. */
export type RailGroup = { title: string | null; commands: Command[] };

/**
 * Returns the rail's groups: one per goal heading of the tool's help, in
 * help order, each with its commands in the guide's order. A tool whose
 * help has no goal headings, such as one built on crumbkit 0.1.0, gets one
 * group with no heading. Usage never reorders the rail.
 *
 * The model gives such a tool one heading named after the tool, so a lone
 * heading with the tool's name is read as no heading. A command under no
 * heading goes into a last group with no heading, so the rail shows every
 * command.
 */
export function railGroups(tool: Tool): RailGroup[] {
  const byPath = new Map(tool.commands.map((each) => [each.path.join(" "), each]));
  const [only] = tool.headings;
  if (tool.headings.length === 1 && only?.title === tool.name) {
    return [{ title: null, commands: guideOrder(tool, only.commands) }];
  }
  const placed = new Set<string>();
  const groups: RailGroup[] = tool.headings.map((heading) => {
    const commands = guideOrder(tool, heading.commands);
    for (const each of commands) {
      placed.add(each.path.join(" "));
    }
    return { title: heading.title, commands };
  });
  const rest = [...byPath.entries()].filter(([path]) => !placed.has(path)).map(([, each]) => each);
  return rest.length === 0 ? groups : [...groups, { title: null, commands: rest }];
}

/** Returns the commands at `paths` in the order of the heading. */
function guideOrder(tool: Tool, paths: string[][]): Command[] {
  return paths.flatMap((path) => {
    const found = tool.commands.find((each) => each.path.join(" ") === path.join(" "));
    return found === undefined ? [] : [found];
  });
}

/** The word for what a command changes, or for a command that starts a program. */
export type Effect =
  | "read-only"
  | "additive"
  | "destructive"
  | "effects unknown"
  | "starts a program";

/**
 * Returns the word for what the command changes, from the class that the
 * server decided.
 */
export function effectOf(command: Command): Effect {
  if (command.effects === null) {
    return "effects unknown";
  }
  return command.class === "read_only" ? "read-only" : command.class;
}

/**
 * Returns true when the command starts a program that the caller names: it
 * declares `runs_program` and is not destructive, whose card comes first.
 * flightdeck cannot preview what such a program changes. A command that only
 * reaches outside the tool (`open_world`) runs with no card.
 */
export function startsProgram(command: Command): boolean {
  return command.class !== "destructive" && command.effects?.runs_program === true;
}

/**
 * Returns true when the command runs only after a confirmation, which the
 * server requires of a destructive command, one whose effects are unknown,
 * and one that starts a program.
 */
export function needsConfirmation(command: Command): boolean {
  return command.class === "destructive" || startsProgram(command);
}

/** Returns true when the command takes crumbkit's `--dry-run`, so the page can preview it. */
export function takesDryRun(command: Command): boolean {
  return command.arguments.some((each) => each.crumbkit_flag === "--dry-run");
}

/**
 * Returns why the page cannot run `command`, or null when it can. Under
 * `--read-only` the server runs only read-only commands.
 */
export function whyNotRunnable(command: Command, readOnly: boolean): string | null {
  if (readOnly && effectOf(command) !== "read-only") {
    return "Turned off by --read-only. Start flightdeck without it to run this command.";
  }
  return null;
}
