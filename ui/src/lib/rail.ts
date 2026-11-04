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

/** The word for what a command changes. */
export type Effect = "read-only" | "additive" | "destructive" | "effects unknown";

/** Returns the word for what the command changes. */
export function effectOf(command: Command): Effect {
  const { effects } = command;
  if (effects === null) {
    return "effects unknown";
  }
  if (effects.read_only) {
    return "read-only";
  }
  return effects.destructive ? "destructive" : "additive";
}

/**
 * Returns true when the command runs only after a preview and a
 * confirmation, which the server requires of a destructive command and of
 * one whose effects are unknown.
 */
export function needsConfirmation(command: Command): boolean {
  const effect = effectOf(command);
  return effect === "destructive" || effect === "effects unknown";
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
