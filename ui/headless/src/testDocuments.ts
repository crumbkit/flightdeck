// Documents the tests build from the compliance build's fixture, each a
// fresh copy, with the changes a test needs and the counts recomputed as the
// build computes them.
import fixture from "../../../tests/fixtures/compliance/compliance.json" with { type: "json" };
import {
  type BranchRun,
  type CellState,
  type ComplianceDocument,
  parseDocument,
  type RepositoryColumn,
} from "./document.ts";

/** Returns a fresh copy of the fixture, parsed. */
export function fixtureDocument(): ComplianceDocument {
  return parseDocument(fixture);
}

const DAY = 86_400;

/** Recounts the document's and each column's cells, as the build does. */
function recount(document: ComplianceDocument): ComplianceDocument {
  const zero = (): Record<CellState, number> => ({
    failing: 0,
    stale: 0,
    not_readable: 0,
    current: 0,
    written: 0,
    absent: 0,
  });
  const total = zero();
  const repositories = document.repositories.map((column, index) => {
    const counts = zero();
    for (const row of document.rows) {
      const cell = row.cells[index];
      if (cell === undefined) continue;
      counts[cell.state] += 1;
      total[cell.state] += 1;
    }
    return { ...column, counts };
  });
  const reports = { current: 0, stale: 0, no_report: 0, invalid_report: 0 };
  for (const column of repositories) reports[column.report] += 1;
  return { ...document, counts: total, reports, repositories };
}

/** Returns the fixture with `name`'s report `days` old, so its current rule cells read stale. */
export function withStaleReport(
  name: string,
  days: number,
  document: ComplianceDocument = fixtureDocument(),
): ComplianceDocument {
  const index = document.repositories.findIndex((column) => column.name === name);
  const repositories = document.repositories.map((column, at) =>
    at === index ? { ...column, report: "stale" as const, age_s: days * DAY + 60 } : column,
  );
  const rows = document.rows.map((row) => ({
    ...row,
    cells: row.cells.map((cell, at) =>
      at === index && row.section !== "suite" && cell.state === "current"
        ? { ...cell, state: "stale" as const }
        : cell,
    ),
  }));
  return recount({ ...document, repositories, rows });
}

/** Returns the fixture with `repository`'s cell of the suite `suite` stale with no run. */
export function withSuiteWithoutRun(suite: string, repository: string): ComplianceDocument {
  const document = fixtureDocument();
  const rows = document.rows.map((row) =>
    row.id === suite
      ? {
          ...row,
          cells: row.cells.map((cell) =>
            cell.repository === repository
              ? { ...cell, state: "stale" as const, run: null, age_s: null }
              : cell,
          ),
        }
      : row,
  );
  return recount({ ...document, rows });
}

/** Returns the fixture with `repository`'s report invalid at `field`. */
export function withInvalidReport(repository: string, field: string): ComplianceDocument {
  const document = fixtureDocument();
  const index = document.repositories.findIndex((column) => column.name === repository);
  const code = "flightdeck::compliance::invalid_report";
  const repositories = document.repositories.map((column, at) =>
    at === index
      ? {
          ...column,
          report: "invalid_report" as const,
          code,
          field,
          message: "unknown variant",
          generated: null,
          age_s: null,
          drift: null,
          source_run: null,
        }
      : column,
  );
  const rows = document.rows.map((row) => ({
    ...row,
    cells: row.cells.map((cell, at) =>
      at === index
        ? {
            ...cell,
            state: "not_readable" as const,
            code,
            result: null,
            facts: null,
            run: null,
            age_s: null,
          }
        : cell,
    ),
  }));
  return recount({ ...document, repositories, rows });
}

/** Returns `document`, the fixture by default, without the column of `name`. */
export function withoutRepository(
  name: string,
  document: ComplianceDocument = fixtureDocument(),
): ComplianceDocument {
  const index = document.repositories.findIndex((column) => column.name === name);
  const repositories = document.repositories.filter((_, at) => at !== index);
  const rows = document.rows.map((row) => ({
    ...row,
    cells: row.cells.filter((_, at) => at !== index),
  }));
  const groups = document.groups.map((group) => ({
    ...group,
    cells: group.cells.filter((_, at) => at !== index),
  }));
  return recount({ ...document, repositories, rows, groups });
}

/** Returns `document` with `repository`'s cell of the rule `rule` current, as a report within its limit gives it. */
export function withPassing(
  repository: string,
  rule: string,
  document: ComplianceDocument = fixtureDocument(),
): ComplianceDocument {
  const rows = document.rows.map((row) =>
    row.id === rule
      ? {
          ...row,
          cells: row.cells.map((cell) =>
            cell.repository === repository
              ? { ...cell, state: "current" as const, result: "within" }
              : cell,
          ),
        }
      : row,
  );
  return recount({ ...document, rows });
}

/**
 * Returns a run of `name`'s workflow on main, `number`, that started
 * `ageS` seconds before the fixture's build, as a fetch record names it.
 */
export function branchRun(
  name: string,
  number: number,
  ageS: number,
  uploaded: boolean,
  document: ComplianceDocument = fixtureDocument(),
): BranchRun {
  const started = new Date(new Date(document.built_at).getTime() - ageS * 1000);
  const id = 9000 + number;
  return {
    run_id: id,
    run_number: number,
    commit: `${String(number).padStart(4, "0")}e19b04f7c2a95d18b3e06f4a1c87d25b9e3f0`,
    branch: "main",
    started_at: started.toISOString().replace(/\.\d{3}Z$/, "Z"),
    url: `https://github.com/${name}/actions/runs/${id}`,
    uploaded,
    age_s: ageS,
  };
}

/** Returns `document`, the fixture by default, with `fields` set on the column of `name`. */
export function withColumn(
  name: string,
  fields: Partial<RepositoryColumn>,
  document: ComplianceDocument = fixtureDocument(),
): ComplianceDocument {
  const repositories = document.repositories.map((column) =>
    column.name === name ? { ...column, ...fields } : column,
  );
  return { ...document, repositories };
}

/**
 * Returns `document`, the fixture by default, as a build writes it when the
 * newest run of `name` uploaded no report and an earlier run's report is
 * beside the fetch record: the column reads no report and names both runs,
 * and each cell its report held reads no report.
 */
export function withSupersededReport(
  name: string,
  document: ComplianceDocument = fixtureDocument(),
): ComplianceDocument {
  const index = document.repositories.findIndex((column) => column.name === name);
  const code = "flightdeck::compliance::no_report";
  const repositories = document.repositories.map((column, at) =>
    at === index
      ? {
          ...column,
          report: "no_report" as const,
          code,
          generated: null,
          age_s: null,
          commit: null,
          committed: null,
          uncommitted: null,
          board_version: null,
          rules_from: null,
          rules_commit: null,
          source_run: null,
          drift: null,
          newest_run: branchRun(name, 12, 3 * 3600, false, document),
          report_run: branchRun(name, 11, 2 * DAY, true, document),
        }
      : column,
  );
  const rows = document.rows.map((row) => ({
    ...row,
    cells: row.cells.map((cell, at) =>
      at === index && cell.state !== "absent"
        ? {
            ...cell,
            state: "not_readable" as const,
            code,
            result: null,
            facts: null,
            run: null,
            age_s: null,
          }
        : cell,
    ),
  }));
  return recount({ ...document, repositories, rows });
}
