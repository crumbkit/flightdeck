// This file is generated. Do not edit it by hand.
// ui/scripts/schemas/zodFromJsonSchema.ts wrote it from tests/golden/schemas/flightdeck-compliance-v1.json.
// Change the source and run `pnpm run schemas` in ui/ to write it again.
import { z } from "../../zod.ts";

/** One finished run of a repository's workflow on its branch, as the fetch
 * record names it. */
export const BranchRunSchema = z.object({
  age_s: z.number().int().nonnegative().nullable(),
  branch: z.string(),
  commit: z.string(),
  run_id: z.number().int().nonnegative(),
  run_number: z.number().int().nonnegative(),
  started_at: z.string(),
  uploaded: z.boolean(),
  url: z.string(),
});

/** The controls job's run that built the document. */
export const BuildRunSchema = z.object({
  label: z.string().nullable(),
  url: z.string().nullable(),
});

/** One pin older than a version rule's minimum. */
export const PinBehindSchema = z.object({
  file: z.string(),
  found: z.string(),
  line: z.number().int().nonnegative(),
});

/** How many items each list of a rule's facts left out. */
export const OmittedSchema = z.object({
  behind: z.number().int().nonnegative(),
  files: z.number().int().nonnegative(),
  over: z.number().int().nonnegative(),
});

/** One file over a per-file budget. */
export const FileOverSchema = z.object({
  file: z.string(),
  used: z.number().int().nonnegative(),
});

/** One value a run recorded for a measure: a number, a label or a flag. */
export const MeasureValueSchema = z.union([
  z.number(),
  z.string(),
  z.boolean(),
]);

/** What a zero rule's suite's latest run reported. */
export const ZeroReadSchema = z.object({
  age_s: z.number().int().nonnegative().nullable(),
  measures: z.record(z.string(), MeasureValueSchema.nullable()),
  run: z.string().nullable(),
  suite: z.string(),
});

/** The facts a rule's checker measured, which a cell copies. */
export const RuleFactsSchema = z.object({
  behind: z.array(PinBehindSchema),
  files: z.array(z.string()),
  max: z.number().int().nonnegative().nullable(),
  omitted: OmittedSchema,
  over: z.array(FileOverSchema),
  used: z.number().int().nonnegative().nullable(),
  zero: ZeroReadSchema.nullable(),
});

/** One case that failed or erred. */
export const FailedCaseSchema = z.object({
  case: z.string(),
  note: z.string().nullable(),
});

/** A suite's newest qualifying run. A cell copies it as it is. */
export const SuiteRunSchema = z.object({
  commit: z.string().nullable(),
  commits_since: z.number().int().nonnegative().nullable(),
  commits_since_code: z.string().nullable(),
  failed: z.array(FailedCaseSchema),
  finished: z.string().nullable(),
  id: z.string(),
  outcome: z.string(),
});

/** What one cell reads. */
export const CellStateSchema = z.enum(["failing", "stale", "not_readable", "current", "written", "absent"]);

/** One rule's or suite's cell in one repository. */
export const CellSchema = z.object({
  age_s: z.number().int().nonnegative().nullable(),
  code: z.string().nullable(),
  facts: RuleFactsSchema.nullable(),
  repository: z.string(),
  result: z.string().nullable(),
  run: SuiteRunSchema.nullable(),
  state: CellStateSchema,
});

/** How many cells read each state. */
export const StateCountsSchema = z.object({
  absent: z.number().int().nonnegative(),
  current: z.number().int().nonnegative(),
  failing: z.number().int().nonnegative(),
  not_readable: z.number().int().nonnegative(),
  stale: z.number().int().nonnegative(),
  written: z.number().int().nonnegative(),
});

/** One group's roll-up in one repository. */
export const GroupCellSchema = z.object({
  counts: z.record(z.string(), z.number().int().nonnegative()).nullable(),
  repository: z.string(),
  state: CellStateSchema,
});

/** One group of rules and its roll-up in each repository. */
export const GroupSchema = z.object({
  cells: z.array(GroupCellSchema),
  id: z.string(),
  members: z.array(z.string()),
  title: z.string(),
});

/** The imposed rule set, as the config names it. */
export const ImposedSetSchema = z.object({
  branch: z.string().nullable().optional(),
  commit: z.string().nullable(),
  from: z.string(),
  name: z.string(),
  root: z.string(),
});

/** How many repositories' reports read each state. */
export const ReportCountsSchema = z.object({
  current: z.number().int().nonnegative(),
  invalid_report: z.number().int().nonnegative(),
  no_report: z.number().int().nonnegative(),
  stale: z.number().int().nonnegative(),
});

/** One imposed rule that the repository holds with different fields. */
export const DiffersSchema = z.object({
  fields: z.array(z.string()),
  id: z.string(),
});

/** Where one repository differs from the imposed rule set. */
export const DriftSchema = z.object({
  behind: z.boolean().nullable(),
  differs: z.array(DiffersSchema),
  extra: z.array(z.string()),
  not_reached: z.array(z.string()),
  other_set: z.boolean(),
});

/** One refusal or warning a report met. */
export const RefusedSchema = z.object({
  code: z.string(),
  message: z.string(),
  rules: z.array(z.string()),
});

/** What a repository's report reads as. */
export const ReportStateSchema = z.enum(["current", "stale", "no_report", "invalid_report"]);

/** The GitHub Actions run that made a report. The document copies it as it
 * is. */
export const SourceRunSchema = z.object({
  commit: z.string(),
  repository: z.string(),
  run_id: z.number().int().nonnegative(),
  run_number: z.number().int().nonnegative(),
});

/** One repository's column. */
export const RepositoryColumnSchema = z.object({
  age_s: z.number().int().nonnegative().nullable(),
  board_version: z.string().nullable(),
  code: z.string().nullable(),
  commit: z.string().nullable(),
  commits_since: z.number().int().nonnegative().nullable().optional(),
  commits_since_code: z.string().nullable().optional(),
  committed: z.string().nullable(),
  counts: StateCountsSchema,
  drift: DriftSchema.nullable(),
  every_hours: z.number().int().nonnegative().nullable().optional(),
  field: z.string().nullable(),
  generated: z.string().nullable(),
  group: z.string().nullable(),
  message: z.string().nullable(),
  name: z.string(),
  newest_run: BranchRunSchema.nullable().optional(),
  refused: z.array(RefusedSchema),
  report: ReportStateSchema,
  report_run: BranchRunSchema.nullable().optional(),
  rules_commit: z.string().nullable(),
  rules_from: z.string().nullable(),
  source_run: SourceRunSchema.nullable(),
  uncommitted: z.boolean().nullable(),
});

/** Which part of the matrix a row sits in. */
export const SectionSchema = z.enum(["measured", "zero", "suite", "written"]);

/** Where a rule comes from. Board's schema allows only these two values,
 * so a report with another is invalid at that rule's `source`. */
export const RuleSourceSchema = z.enum(["imposed", "repository"]);

/** One rule or suite and its cell in each repository. */
export const RowSchema = z.object({
  cases: z.number().int().nonnegative().nullable(),
  cells: z.array(CellSchema),
  checker: z.unknown(),
  groups: z.array(z.string()),
  id: z.string(),
  kind: z.string().nullable(),
  paths: z.array(z.string()).nullable().optional(),
  purpose: z.string().nullable(),
  requirement: z.string().nullable(),
  section: SectionSchema,
  show: z.string().nullable(),
  source: RuleSourceSchema.nullable(),
});

/** The files that `--out` wrote, as the printed document names them. */
export const SiteFilesSchema = z.object({
  document: z.string(),
  page: z.string(),
  page_bytes: z.number().int().nonnegative(),
  warnings: z.array(z.string()),
});

/** The compliance of every repository the config names, as the pages draw
 * it. */
export const ComplianceDocumentSchema = z.object({
  build_run: BuildRunSchema.nullable(),
  built_at: z.string(),
  counts: StateCountsSchema,
  groups: z.array(GroupSchema),
  imposed: ImposedSetSchema,
  next_build_at: z.string().nullable(),
  reports: ReportCountsSchema,
  repositories: z.array(RepositoryColumnSchema),
  rows: z.array(RowSchema),
  site: SiteFilesSchema.nullable().optional(),
  stale_after_days: z.number().int().nonnegative(),
  title: z.string(),
});

/** Each repository's report as a column and each rule and suite as a row, with each cell's state, each group's roll-up and where each repository differs from the imposed rule set. */
export const ComplianceEnvelopeSchema = z.object({
  data: ComplianceDocumentSchema,
  dry_run: z.literal(true).optional(),
  failures: z.array(z.object({
    alternatives: z.array(z.union([
      z.object({
        all: z.string().optional(),
        call: z.string(),
        kind: z.literal("run"),
      }),
      z.object({
        call: z.string(),
        kind: z.literal("terminal"),
      }),
      z.object({
        args: z.array(z.string()),
        kind: z.literal("program"),
        program: z.string(),
        run: z.string(),
      }),
      z.object({
        kind: z.literal("retry"),
      }),
      z.object({
        kind: z.literal("after"),
        repair: z.string(),
        then: z.union([
          z.object({
            all: z.string().optional(),
            call: z.string(),
            kind: z.literal("run"),
          }),
          z.object({
            call: z.string(),
            kind: z.literal("terminal"),
          }),
          z.object({
            kind: z.literal("retry"),
          }),
        ]),
      }),
      z.object({
        kind: z.literal("hint"),
        text: z.string(),
      }),
      z.object({
        kind: z.literal("none"),
        reason: z.string(),
      }),
    ])).optional(),
    cause: z.string().optional(),
    causes: z.array(z.string()).optional(),
    code: z.string().regex(new RegExp("^[!-~]+$")),
    fields: z.record(z.string().regex(new RegExp("^[a-z0-9_]+$")), z.union([
      z.string(),
      z.number().int(),
      z.boolean(),
      z.array(z.unknown()),
      z.record(z.string(), z.unknown()),
    ]).nullable()).optional(),
    index: z.number().int().nonnegative().optional(),
    key: z.string().optional(),
    labels: z.array(z.object({
      column: z.number().int().min(1).optional(),
      excerpt: z.object({
        line: z.number().int().min(1),
        text: z.string(),
      }).optional(),
      length: z.number().int().nonnegative(),
      line: z.number().int().min(1).optional(),
      name: z.string().optional(),
      offset: z.number().int().nonnegative(),
      text: z.string().optional(),
    })).optional(),
    message: z.string(),
    recovery: z.union([
      z.object({
        all: z.string().optional(),
        call: z.string(),
        kind: z.literal("run"),
      }),
      z.object({
        call: z.string(),
        kind: z.literal("terminal"),
      }),
      z.object({
        args: z.array(z.string()),
        kind: z.literal("program"),
        program: z.string(),
        run: z.string(),
      }),
      z.object({
        kind: z.literal("retry"),
      }),
      z.object({
        kind: z.literal("after"),
        repair: z.string(),
        then: z.union([
          z.object({
            all: z.string().optional(),
            call: z.string(),
            kind: z.literal("run"),
          }),
          z.object({
            call: z.string(),
            kind: z.literal("terminal"),
          }),
          z.object({
            kind: z.literal("retry"),
          }),
        ]),
      }),
      z.object({
        kind: z.literal("hint"),
        text: z.string(),
      }),
      z.object({
        kind: z.literal("none"),
        reason: z.string(),
      }),
    ]),
    related: z.array(z.object({
      cause: z.string().optional(),
      code: z.string().optional(),
      help: z.string().optional(),
      labels: z.array(z.object({
        column: z.number().int().min(1).optional(),
        excerpt: z.object({
          line: z.number().int().min(1),
          text: z.string(),
        }).optional(),
        length: z.number().int().nonnegative(),
        line: z.number().int().min(1).optional(),
        name: z.string().optional(),
        offset: z.number().int().nonnegative(),
        text: z.string().optional(),
      })).optional(),
      message: z.string(),
      related: z.array(z.unknown()).optional(),
      severity: z.enum(["error", "warning", "advice"]),
      url: z.string().optional(),
    })).optional(),
    sentence: z.string().optional(),
    url: z.string().optional(),
  })),
  schema: z.literal("flightdeck/compliance/v1"),
});
