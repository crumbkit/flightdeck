import { type Command, ModelReply, type Tool } from "@/api/model";
import environment from "./shelf-environment.json" with { type: "json" };
import model from "./shelf-model.json" with { type: "json" };

/**
 * shelf's model as `GET /api/model` sends it, with the five goal headings
 * of shelf's root help in help order. shelf is built on crumbkit 0.3.2, so
 * every command is counted and the server reads its built-in marks from
 * `commands list`.
 */
export const shelfReply: ModelReply = ModelReply.parse(model);
export const shelf: Tool = shelfReply.tool;

/** The queries of the usage log that flightdeck's loader runs. */
const loaderQueries = ["self figures list", "self usage", "self runs list"];

/**
 * shelf's model as the server sends it for a tool on a crumbkit before
 * 0.3.2, whose usage reports cannot leave out flightdeck's own runs: the
 * loader's three queries of the usage log are not counted.
 */
export const olderShelf: Tool = {
  ...shelf,
  commands: shelf.commands.map((each) =>
    loaderQueries.includes(each.path.join(" ")) ? { ...each, counted: false } : each,
  ),
};

/**
 * shelf's model with the headings that a tool on crumbkit 0.1.0 gives,
 * whose help prints no goal headings: one heading named after the tool
 * holds every command.
 */
export const shelfWithoutGoals: Tool = {
  ...shelf,
  headings: [{ title: shelf.name, commands: shelf.commands.map((each) => each.path) }],
};

/**
 * The shelf of tests/fixtures/environment, which mounts crumbkit-starmap's
 * `self environment list`, as the server sends its model: shelf's model with
 * that command and the server's reading of the command's document, which
 * the journey "environment shelf" checks against the real server.
 */
export const environmentShelf: Tool = ModelReply.shape.tool.parse({
  ...shelf,
  commands: [
    ...shelf.commands,
    {
      ...command(shelf, "self usage"),
      path: ["self", "environment", "list"],
      about: "List the environment variables that shelf declares and their values",
      arguments: command(shelf, "self usage").arguments.filter((each) => each.global),
      examples: [],
      cost: null,
      runs: 0,
    },
  ],
  environment,
});

/** Returns the tool's command at `path`, such as "books list", or throws. */
export function command(tool: Tool, path: string): Command {
  const found = tool.commands.find((candidate) => candidate.path.join(" ") === path);
  if (found === undefined) {
    throw new Error(`${tool.name} has no command ${path}`);
  }
  return found;
}
