// The `flightdeck/compliance/v1` document that `flightdeck compliance build`
// writes to compliance.json, as a zod schema and the types it parses to. The
// fields mirror src/compliance/document.rs; a field the build adds later is
// dropped here until the pages read it.
import { z } from "zod";

// zod compiles fast parsers with `new Function` when the page allows it, and
// its probe for that permission breaks a Content-Security-Policy that forbids
// eval (EMB-6). The setting applies before any schema below is built.
z.config({ jitless: true });

/** The schema name the document's envelope carries. */
export const SCHEMA = "flightdeck/compliance/v1";

const cellStateSchema = z.enum([
  "failing",
  "stale",
  "not_readable",
  "current",
  "written",
  "absent",
]);

/** What one cell reads, as the build spells it. */
export type CellState = z.infer<typeof cellStateSchema>;

/** Every cell state, worst first, as src/compliance/state.rs orders them. */
export const CELL_STATES_WORST_FIRST: readonly CellState[] = cellStateSchema.options;

const stateCountsSchema = z.object({
  failing: z.number().int().nonnegative(),
  stale: z.number().int().nonnegative(),
  not_readable: z.number().int().nonnegative(),
  current: z.number().int().nonnegative(),
  written: z.number().int().nonnegative(),
  absent: z.number().int().nonnegative(),
});

/** How many cells read each state. */
export type StateCounts = z.infer<typeof stateCountsSchema>;

const reportStateSchema = z.enum(["current", "stale", "no_report", "invalid_report"]);

/** What a repository's report reads as. */
export type ReportState = z.infer<typeof reportStateSchema>;

const sourceRunSchema = z.object({
  repository: z.string(),
  run_id: z.number().int(),
  run_number: z.number().int(),
  commit: z.string(),
});

const differsSchema = z.object({ id: z.string(), fields: z.array(z.string()) });

const driftSchema = z.object({
  extra: z.array(z.string()),
  differs: z.array(differsSchema),
  not_reached: z.array(z.string()),
  other_set: z.boolean(),
  behind: z.boolean().nullable(),
});

/** Where one repository differs from the imposed rule set. */
export type Drift = z.infer<typeof driftSchema>;

const refusedSchema = z.object({
  code: z.string(),
  rules: z.array(z.string()),
  message: z.string(),
});

const repositorySchema = z.object({
  name: z.string(),
  group: z.string().nullable(),
  report: reportStateSchema,
  code: z.string().nullable(),
  field: z.string().nullable(),
  message: z.string().nullable(),
  generated: z.string().nullable(),
  age_s: z.number().int().nonnegative().nullable(),
  commit: z.string().nullable(),
  committed: z.string().nullable(),
  uncommitted: z.boolean().nullable(),
  board_version: z.string().nullable(),
  rules_from: z.string().nullable(),
  rules_commit: z.string().nullable(),
  source_run: sourceRunSchema.nullable(),
  drift: driftSchema.nullable(),
  refused: z.array(refusedSchema),
  counts: stateCountsSchema,
});

/** One repository's column. */
export type RepositoryColumn = z.infer<typeof repositorySchema>;

const sectionSchema = z.enum(["measured", "zero", "suite", "written"]);

/** Which part of the matrix a row sits in. */
export type Section = z.infer<typeof sectionSchema>;

const measureValueSchema = z.union([z.number(), z.string(), z.boolean()]);

const zeroReadSchema = z.object({
  suite: z.string(),
  run: z.string().nullable(),
  age_s: z.number().int().nonnegative().nullable(),
  measures: z.record(z.string(), measureValueSchema.nullable()),
});

const factsSchema = z.object({
  used: z.number().int().nonnegative().nullable(),
  max: z.number().int().nonnegative().nullable(),
  files: z.array(z.string()),
  over: z.array(z.object({ file: z.string(), used: z.number().int().nonnegative() })),
  behind: z.array(
    z.object({ file: z.string(), line: z.number().int().positive(), found: z.string() }),
  ),
  omitted: z.object({
    files: z.number().int().nonnegative(),
    over: z.number().int().nonnegative(),
    behind: z.number().int().nonnegative(),
  }),
  zero: zeroReadSchema.nullable(),
});

/** What a rule's checker measured in one repository. */
export type RuleFacts = z.infer<typeof factsSchema>;

const suiteRunSchema = z.object({
  id: z.string(),
  finished: z.string().nullable(),
  commit: z.string().nullable(),
  outcome: z.string(),
  failed: z.array(z.object({ case: z.string(), note: z.string().nullable() })),
  commits_since: z.number().int().nonnegative().nullable(),
  commits_since_code: z.string().nullable(),
});

/** A suite's newest qualifying run. */
export type SuiteRun = z.infer<typeof suiteRunSchema>;

const cellSchema = z.object({
  repository: z.string(),
  state: cellStateSchema,
  code: z.string().nullable(),
  result: z.string().nullable(),
  facts: factsSchema.nullable(),
  run: suiteRunSchema.nullable(),
  age_s: z.number().int().nonnegative().nullable(),
});

/** One rule's or suite's cell in one repository. */
export type Cell = z.infer<typeof cellSchema>;

const checkerSchema = z.record(z.string(), z.unknown());

/** What a program checks, as the report gives it, such as `{ check: "budget", max: 60 }`. */
export type Checker = z.infer<typeof checkerSchema>;

const rowSchema = z.object({
  id: z.string(),
  section: sectionSchema,
  source: z.enum(["imposed", "repository"]).nullable(),
  requirement: z.string().nullable(),
  purpose: z.string().nullable(),
  kind: z.string().nullable(),
  show: z.string().nullable(),
  checker: checkerSchema.nullable(),
  groups: z.array(z.string()),
  cases: z.number().int().nonnegative().nullable(),
  cells: z.array(cellSchema),
});

/** One rule or suite and its cell in each repository. */
export type Row = z.infer<typeof rowSchema>;

const groupSchema = z.object({
  id: z.string(),
  title: z.string(),
  members: z.array(z.string()),
  cells: z.array(
    z.object({
      repository: z.string(),
      state: cellStateSchema,
      counts: z.record(z.string(), z.number().int().nonnegative()).nullable(),
    }),
  ),
});

/** One group of rules and its roll-up in each repository. */
export type Group = z.infer<typeof groupSchema>;

const dataSchema = z.object({
  title: z.string(),
  built_at: z.string(),
  next_build_at: z.string().nullable(),
  stale_after_days: z.number().int().nonnegative(),
  build_run: z.object({ label: z.string().nullable(), url: z.string().nullable() }).nullable(),
  imposed: z.object({
    name: z.string(),
    from: z.string(),
    root: z.string(),
    commit: z.string().nullable(),
  }),
  counts: stateCountsSchema,
  reports: z.object({
    current: z.number().int().nonnegative(),
    stale: z.number().int().nonnegative(),
    no_report: z.number().int().nonnegative(),
    invalid_report: z.number().int().nonnegative(),
  }),
  repositories: z.array(repositorySchema),
  rows: z.array(rowSchema),
  groups: z.array(groupSchema),
});

/** The compliance of every repository the build's config names. */
export type ComplianceDocument = z.infer<typeof dataSchema>;

const envelopeSchema = z.object({
  schema: z.literal(SCHEMA),
  data: dataSchema,
  failures: z.array(z.unknown()),
});

/** The document did not parse: `path` names the first field that failed, such as `data.rows.0.section`. */
export class InvalidDocument extends Error {
  readonly code = "invalid_document";
  readonly path: string;

  constructor(path: string, message: string, options?: ErrorOptions) {
    super(path === "" ? message : `${path}: ${message}`, options);
    this.name = "InvalidDocument";
    this.path = path;
  }
}

/**
 * Returns the document that `json`, the parsed compliance.json, holds. It
 * throws InvalidDocument, with the zod error as its cause, when the schema
 * is not `flightdeck/compliance/v1` or a field does not parse, so a page
 * never renders half a document.
 */
export function parseDocument(json: unknown): ComplianceDocument {
  const parsed = envelopeSchema.safeParse(json);
  if (parsed.success) return parsed.data.data;
  const issue = parsed.error.issues[0];
  const path = issue === undefined ? "" : issue.path.map(String).join(".");
  throw new InvalidDocument(path, issue?.message ?? "not a compliance document", {
    cause: parsed.error,
  });
}
