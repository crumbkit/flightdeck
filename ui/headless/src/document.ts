// The `flightdeck/compliance/v1` document that `flightdeck compliance build`
// writes to compliance.json, as the zod schemas and the types they parse to.
// The schemas are generated from the JSON Schema that flightdeck publishes
// (api/generated/compliance.ts), so the Rust types stay their one home. A
// field the build adds later is dropped here until the pages read it.
import {
  type CellSchema,
  CellStateSchema,
  ComplianceDocumentSchema,
  ComplianceEnvelopeSchema,
  type DriftSchema,
  type GroupSchema,
  type ReportStateSchema,
  type RepositoryColumnSchema,
  RowSchema,
  type RuleFactsSchema,
  type SectionSchema,
  type StateCountsSchema,
  type SuiteRunSchema,
} from "./api/generated/compliance.ts";
import { z } from "./zod.ts";

/** The schema name the document's envelope carries. */
export const SCHEMA = ComplianceEnvelopeSchema.shape.schema.value;

/** What one cell reads, as the build spells it. */
export type CellState = z.infer<typeof CellStateSchema>;

/** Every cell state, worst first, as src/compliance/state.rs orders them. */
export const CELL_STATES_WORST_FIRST: readonly CellState[] = CellStateSchema.options;

/** How many cells read each state. */
export type StateCounts = z.infer<typeof StateCountsSchema>;

/** What a repository's report reads as. */
export type ReportState = z.infer<typeof ReportStateSchema>;

/** Where one repository differs from the imposed rule set. */
export type Drift = z.infer<typeof DriftSchema>;

/** One repository's column. */
export type RepositoryColumn = z.infer<typeof RepositoryColumnSchema>;

/** Which part of the matrix a row sits in. */
export type Section = z.infer<typeof SectionSchema>;

/** What a rule's checker measured in one repository. */
export type RuleFacts = z.infer<typeof RuleFactsSchema>;

/** A suite's newest qualifying run. */
export type SuiteRun = z.infer<typeof SuiteRunSchema>;

/** One rule's or suite's cell in one repository. */
export type Cell = z.infer<typeof CellSchema>;

// The published schema lets a checker be any JSON value, because the build
// copies it from a report as it is. The pages read its fields by name, so
// the package admits only an object or null, as it always has.
const CheckerSchema = z.record(z.string(), z.unknown());

/** What a program checks, as the report gives it, such as `{ check: "budget", max: 60 }`. */
export type Checker = z.infer<typeof CheckerSchema>;

const PageRowSchema = RowSchema.extend({ checker: CheckerSchema.nullable() });

/** One rule or suite and its cell in each repository. */
export type Row = z.infer<typeof PageRowSchema>;

/** One group of rules and its roll-up in each repository. */
export type Group = z.infer<typeof GroupSchema>;

const PageDocumentSchema = ComplianceDocumentSchema.extend({ rows: z.array(PageRowSchema) });

/** The compliance of every repository the build's config names. */
export type ComplianceDocument = z.infer<typeof PageDocumentSchema>;

const PageEnvelopeSchema = ComplianceEnvelopeSchema.extend({ data: PageDocumentSchema });

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
  const parsed = PageEnvelopeSchema.safeParse(json);
  if (parsed.success) return parsed.data.data;
  const issue = parsed.error.issues[0];
  const path = issue === undefined ? "" : issue.path.map(String).join(".");
  throw new InvalidDocument(path, issue?.message ?? "not a compliance document", {
    cause: parsed.error,
  });
}
