import { type Command, ModelReply, type Tool } from "@/api/model";
import model from "./shelf-model.json" with { type: "json" };

/**
 * shelf's model as `GET /api/model` sends it. crumbkit 0.1.0's shelf prints
 * no goal headings, so the model has one heading named after the tool.
 */
export const shelfReply: ModelReply = ModelReply.parse(model);
export const shelf: Tool = shelfReply.tool;

/**
 * shelf's model with the goal headings that a newer crumbkit prints, in the
 * order of shelf's root help.
 */
export const shelfWithGoals: Tool = {
  ...shelf,
  headings: [
    {
      title: "Browse the catalogue",
      commands: [
        ["books", "list"],
        ["books", "show"],
        ["books", "summarize"],
        ["books", "remove"],
      ],
    },
    {
      title: "Explore the commands",
      commands: [
        ["commands", "list"],
        ["schemas", "list"],
        ["schemas", "show"],
      ],
    },
    {
      title: "Review how shelf is used",
      commands: [
        ["self", "usage"],
        ["self", "figures", "list"],
      ],
    },
  ],
};

/** Returns the tool's command at `path`, such as "books list", or throws. */
export function command(tool: Tool, path: string): Command {
  const found = tool.commands.find((candidate) => candidate.path.join(" ") === path);
  if (found === undefined) {
    throw new Error(`${tool.name} has no command ${path}`);
  }
  return found;
}
