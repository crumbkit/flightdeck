import type { Command, CommandPath, Link, Tool } from "@/api/model";
import { type Draft, emptyDraft, setValue } from "./form";

/** The column whose cells an inferred link starts from. */
const idColumn = "id";

function samePath(left: CommandPath, right: CommandPath): boolean {
  return left.join(" ") === right.join(" ");
}

/**
 * Returns the link that a cell of the `id` column of `command`'s rows
 * opens: the first link that the model infers from the command, or null.
 */
export function cellLinkOf(tool: Tool, command: Command): Link | null {
  return tool.links.find((link) => samePath(link.source, command.path)) ?? null;
}

/** Returns true when `column` is the column that an inferred link starts from. */
export function isLinkColumn(column: string): boolean {
  return column === idColumn;
}

/**
 * Returns the tooltip and accessible name of an id cell, such as "Opens
 * books show with id 2 · inferred from the books resource".
 */
export function cellLinkWords(link: Link, value: string): string {
  const resource = link.target.slice(0, -1).join(" ");
  return `Opens ${link.target.join(" ")} with ${idColumn} ${value} · inferred from the ${resource} resource`;
}

/**
 * Returns the link whose source lists the choices of `argument` of
 * `target`: the first link that the model infers to it whose source is
 * read-only and needs no value of its own, or null. The server runs only
 * a read-only source.
 */
export function chooserLinkOf(tool: Tool, target: Command, argument: string): Link | null {
  return (
    tool.links.find((link) => {
      if (!samePath(link.target, target.path) || link.argument !== argument) {
        return false;
      }
      const source = tool.commands.find((each) => samePath(each.path, link.source));
      return (
        source !== undefined &&
        source.effects?.read_only === true &&
        !source.effects.destructive &&
        !source.arguments.some((each) => each.required)
      );
    }) ?? null
  );
}

/** Returns the chooser's footer, such as "From shelf books list, column id · inferred". */
export function chooserFooter(tool: Tool, link: Link): string {
  return `From ${tool.name} ${link.source.join(" ")}, column ${idColumn} · inferred`;
}

/**
 * Returns the draft of `target` with `value` in its argument `name`, as a
 * one-item list when the argument repeats.
 */
export function filled(target: Command, name: string, value: string): Draft {
  const argument = target.arguments.find((each) => each.name === name);
  return setValue(emptyDraft(target), name, argument?.arity.kind === "many" ? [value] : value);
}
