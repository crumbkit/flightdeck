import { type Command, ModelReply, type Tool } from "@/api/model";
import model from "./shelf-model.json" with { type: "json" };

/**
 * shelf's model as `GET /api/model` sends it, with the three goal headings
 * of shelf's root help in help order.
 */
export const shelfReply: ModelReply = ModelReply.parse(model);
export const shelf: Tool = shelfReply.tool;

/**
 * shelf's model as a tool on crumbkit 0.1.0 gives it, whose help prints no
 * goal headings: one heading named after the tool holds every command.
 */
export const shelfWithoutGoals: Tool = {
  ...shelf,
  headings: [{ title: shelf.name, commands: shelf.commands.map((each) => each.path) }],
};

/** Returns the tool's command at `path`, such as "books list", or throws. */
export function command(tool: Tool, path: string): Command {
  const found = tool.commands.find((candidate) => candidate.path.join(" ") === path);
  if (found === undefined) {
    throw new Error(`${tool.name} has no command ${path}`);
  }
  return found;
}
