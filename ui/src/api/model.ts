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
  /** Whether every command takes the flag, as `--json` and `--no-input` are. */
  global: z.boolean(),
  /** Whether the argument takes its values after `--`, as clap's `last` does. */
  after_double_dash: z.boolean(),
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

/**
 * What a command changes, or null for a command that declares nothing.
 * `open_world` marks a command that reaches systems outside the tool, and
 * `runs_program` one that starts a program the caller names.
 */
const Effects = z
  .object({
    read_only: z.boolean(),
    destructive: z.boolean(),
    idempotent: z.boolean(),
    open_world: z.boolean(),
    runs_program: z.boolean(),
  })
  .nullable();
export type Effects = z.infer<typeof Effects>;

/** How a command's effects decide how it starts, which the server decides. */
const Class = z.enum(["read_only", "additive", "destructive"]);

/** A day of the calendar, which the server checked. */
const Day = z.object({
  year: z.number().int(),
  month: z.number().int().min(1).max(12),
  day: z.number().int().min(1).max(31),
});
export type Day = z.infer<typeof Day>;

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
  p50_ms: Figure,
  p95_ms: Figure,
  p50_out_bytes: Figure,
  p95_out_bytes: Figure,
});
export type Figures = z.infer<typeof Figures>;

/** What a finding of the tool's `self usage` is, as the server names it. */
const FindingKind = z.enum([
  "retry_after_failure",
  "identical_repeat",
  "slow_command",
  "help_loop",
  "usage_error",
  "heavy_output",
  "other",
]);

/** One finding of the tool's `self usage` about a command, with the figures that describe it. */
const Finding = z.object({
  kind: FindingKind,
  /** The runs that show the pattern; a help loop counts its help pages. */
  runs: z.number().int().nonnegative(),
  /** The code of the failure that the runs retried, without the tool's name in front. */
  code: z.string().nullable(),
  p95_ms: Figure,
  p95_out_bytes: Figure,
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
  /**
   * Whether the figures count the command's runs. The loader's own
   * queries, such as `commands list`, are not counted: the figures leave
   * out every run of them, because they cannot tell flightdeck's runs from
   * an agent's.
   */
  counted: z.boolean(),
  /**
   * Whether crumbkit adds the command to every tool, such as `self usage`.
   * The server judges it by the command's first word, so a tool's own
   * command under `commands`, `schemas`, `self` or `aliases` counts too.
   */
  built_in: z.boolean(),
  /**
   * Whether `commands list` marks the command as one that prints no
   * result. Its runs report no output size, which means no output.
   */
  prints_no_result: z.boolean(),
});
export type Command = z.infer<typeof Command>;

/**
 * One run in the tool's usage log, from the tool's `self runs list`, in
 * order within its session. The log keeps flag names, never their values.
 */
const Run = z.object({
  /** The run's session, numbered from 1. */
  session: z.number().int().positive(),
  /** The run's place in its session, from 1. */
  step: z.number().int().positive(),
  /** When the run ended, in whole seconds since the Unix epoch. */
  at: z.number().int().nonnegative(),
  /** The agent that ran it, such as "claude-code". */
  caller: z.string(),
  /** The command's words below the tool's name, such as "books show". */
  command: z.string(),
  flags: z.array(z.string()),
  /** Whether the run printed a help page. */
  help: z.boolean(),
  /** The exit code, or null when the run ended before it exited. */
  exit: z.number().int().min(0).max(255).nullable(),
  /** The failure's code without the tool's name in front, such as "books::not_found". */
  code: z.string().nullable(),
  ms: z.number().int().nonnegative(),
  /** The bytes written to standard output, or null when the run did not finish. */
  out_bytes: Figure,
});
export type Run = z.infer<typeof Run>;

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

/** What happens when a variable has no value in one mode. */
const Rule = z.discriminatedUnion("rule", [
  z.object({ rule: z.literal("required") }),
  z.object({ rule: z.literal("optional") }),
  z.object({ rule: z.literal("default"), default: z.string() }),
]);
export type Rule = z.infer<typeof Rule>;

/**
 * The words of crumbkit-starmap's closed list of types. A word that a later
 * crumbkit-starmap adds reads as "other", as it does on the server.
 */
export const typeNames = [
  "path",
  "port",
  "url",
  "socket_address",
  "boolean",
  "integer",
  "enumeration",
  "string",
  "bytes",
  "other",
] as const;

/** The words of crumbkit-starmap's closed list of sources, read as the type names are. */
export const sourceKinds = [
  "setting",
  "secret",
  "generated",
  "inherited",
  "provided",
  "other",
] as const;

/**
 * One environment variable from the tool's `self environment list`, with
 * the field names of that document. A withheld value never reaches the
 * page: the server drops the text of any value whose `show` is not
 * "shown", and crumbkit-starmap prints none.
 */
const Variable = z.object({
  name: z.string().min(1),
  /** The value's type, such as "port" or "log-level: one of error, warn, info, debug". */
  type: z.string(),
  /** The one word that names the type, or null from a tool before crumbkit-starmap 0.4.2. */
  type_name: z.enum(typeNames).catch("other").nullable(),
  /** What happens when the variable is unset, such as "default 8080" or "optional; production required". */
  unset: z.string(),
  /**
   * The unset rule in local mode and, only when it differs, in production,
   * or null from a tool before crumbkit-starmap 0.4.2.
   */
  unset_rule: z.object({ local: Rule, production: Rule.nullable() }).nullable(),
  /** Where the value comes from, such as "setting" or "secret, 1Password". */
  source: z.string(),
  /**
   * The source's kind and the kind of its bind, such as "1Password", or
   * null from a tool before crumbkit-starmap 0.4.2.
   */
  source_kind: z
    .object({
      kind: z.enum(sourceKinds).catch("other"),
      bind: z.string().nullable(),
    })
    .nullable(),
  /** The value as flightdeck started the tool: its state, and its text unless it is unset or withheld. */
  value: z.object({
    state: z.enum(["set", "default", "unset", "invalid"]),
    text: z.string().nullable(),
  }),
  /** "shown" or "withheld", or null when the tool leaves it out. */
  show: z.string().nullable(),
  /** The variable's bind, such as a 1Password reference, or null. */
  bind: z.string().nullable(),
  purpose: z.string().nullable(),
  /** What the tool does when the variable is unset, or null when the declaration says nothing. */
  effect: z.string().nullable(),
  /** Each value of a named enumeration with what it means, or null for any other type. */
  values: z.array(z.object({ value: z.string(), description: z.string() })).nullable(),
  /** The check that the tool's own code makes of the value, or null. */
  check: z.string().nullable(),
  /** The group of variables that are set together, or null. */
  group: z.string().nullable(),
  /** The library that reads the variable in every tool, such as "crumbkit", or null for the tool's own. */
  owner: z.string().nullable(),
});
export type Variable = z.infer<typeof Variable>;

const Tool = z.object({
  name: z.string().min(1),
  version: z.string(),
  about: z.string().nullable(),
  headings: z.array(Heading),
  commands: z.array(Command),
  links: z.array(Link),
  /** The day the tool's figures start, or null when no command has a cost. */
  figures_since: Day.nullable(),
  /** Each run in the tool's usage log, or null when the tool reports none. */
  history: z.array(Run).nullable(),
  /**
   * Each variable the tool declares and each that crumbkit reads, or null
   * when the tool does not mount `self environment list` or it failed.
   */
  environment: z.array(Variable).nullable(),
});
export type Tool = z.infer<typeof Tool>;

export const ModelReply = z.object({
  /** The directory flightdeck started in, or null when the page cannot send it back. */
  directory: z.string().nullable(),
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
