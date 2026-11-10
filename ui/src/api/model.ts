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

/** A figure that the tool may not report: null when it does not. */
const Figure = z.number().int().nonnegative().nullable();

/**
 * The figures of a command's runs from the tool's `self figures list`. The
 * server sends null for each figure the tool does not report.
 */
const Figures = z.object({
  /** The runs that printed no help page. */
  runs: z.number().int().nonnegative(),
  /** The runs that printed the command's help page. */
  help_pages: Figure,
  /** The help pages that sessions read before their runs, summed. */
  help_before: Figure,
  /** The runs whose command line the parser refused. */
  refused: Figure,
  failed: Figure,
  p95_ms: Figure,
  p95_out_bytes: Figure,
});
export type Figures = z.infer<typeof Figures>;

/** One finding of the tool's `self usage` about a command. */
const Finding = z.object({
  /** What the pattern is, such as "retry after failure". */
  finding: z.string().min(1),
  runs: z.number().int().nonnegative(),
  suggestion: z.string(),
  /** The code of the failure that the runs retried, for a retry after failure. */
  code: z.string().nullable(),
});
export type Finding = z.infer<typeof Finding>;

/**
 * What a command costs the agents that call it. `findings` is null when the
 * tool's `self usage` failed or the tool does not mount it.
 */
const Cost = z.object({
  figures: Figures,
  findings: z.array(Finding).nullable(),
});
export type Cost = z.infer<typeof Cost>;

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
  /** What the command costs an agent, or null when the figures hold no row for it. */
  cost: Cost.nullable(),
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
  /** The day the tool's figures start, such as "2026-11-03", or null. */
  figures_since: z.iso.date().nullable(),
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
