import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { getJson } from "./client";

// The tool's model as `GET /api/model` sends it. Each schema reads the
// fields the page draws and drops the rest, such as the documents' JSON
// Schemas.

/** A command's words below the tool's name, such as ["books", "list"]. */
export const CommandPath = z.array(z.string()).min(1);
export type CommandPath = z.infer<typeof CommandPath>;

const Arity = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("flag") }),
  z.object({ kind: z.literal("count") }),
  z.object({ kind: z.literal("one"), value_names: z.array(z.string()) }),
  z.object({ kind: z.literal("many"), value_names: z.array(z.string()) }),
]);

const Argument = z.object({
  /** `--limit` for an option, `<ID>` for a positional argument. */
  name: z.string().min(1),
  short: z.string().nullable(),
  kind: z.enum(["named", "positional"]),
  arity: Arity,
  required: z.boolean(),
  defaults: z.array(z.string()),
  possible_values: z.array(z.string()),
  help: z.object({ line: z.string().nullable(), long: z.string().nullable() }),
  one_of: z.array(z.string()),
  /** Which of crumbkit's own flags the argument is, such as `--json`. */
  crumbkit_flag: z.string().nullable(),
});
export type Argument = z.infer<typeof Argument>;

const Example = z.object({
  /** The whole command line, such as `shelf books show 2 5`. */
  command: z.string(),
  about: z.string(),
});
export type Example = z.infer<typeof Example>;

const Column = z.object({
  name: z.string().min(1),
  long: z.boolean(),
  description: z.string().nullable(),
});
export type Column = z.infer<typeof Column>;

/** What a command changes, or null for a command that declares nothing. */
const Effects = z
  .object({ read_only: z.boolean(), destructive: z.boolean(), idempotent: z.boolean() })
  .nullable();
export type Effects = z.infer<typeof Effects>;

/** How a command's effects decide how it starts, which the server decides. */
const Class = z.enum(["read_only", "additive", "destructive"]);

const Command = z.object({
  path: CommandPath,
  about: z.string().nullable(),
  arguments: z.array(Argument),
  examples: z.array(Example),
  result: z.object({
    form: z.enum(["document", "stream", "bytes", "headed"]).nullable(),
    columns: z.array(Column),
  }),
  effects: Effects,
  /** What the server makes of the effects: it counts a command that declares no effects as destructive. */
  class: Class,
  /** How often the command ran, from the tool's usage figures, or 0. */
  runs: z.number().int().nonnegative(),
});
export type Command = z.infer<typeof Command>;

const Heading = z.object({
  title: z.string(),
  commands: z.array(CommandPath),
});

/**
 * A link that the model inferred: the `id` column of the source's rows
 * fills the target's argument, such as `<ID>`.
 */
const Link = z.object({
  source: CommandPath,
  target: CommandPath,
  argument: z.string(),
});
export type Link = z.infer<typeof Link>;

const Tool = z.object({
  name: z.string().min(1),
  version: z.string(),
  about: z.string().nullable(),
  headings: z.array(Heading),
  commands: z.array(Command),
  links: z.array(Link),
});
export type Tool = z.infer<typeof Tool>;

export const ModelReply = z.object({
  read_only: z.boolean(),
  tool: Tool,
});
export type ModelReply = z.infer<typeof ModelReply>;

/** The model query (APP-10). The model never changes while the server runs. */
export function modelQuery() {
  return queryOptions({
    queryKey: ["model"],
    queryFn: ({ signal }) => getJson("/api/model", ModelReply, signal),
    staleTime: Number.POSITIVE_INFINITY,
  });
}
