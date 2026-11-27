// The sample document the Compliance site's tests draw: the repositories,
// rules and suites of the signed artboards 52, 54 and 55, as a compliance
// build would write them.
import { type ComplianceDocument, parseDocument } from "@crumbkit/flightdeck-headless/document";
import sample from "./sample.json" with { type: "json" };

/** The sample as compliance.json holds it. */
export const sampleJson: unknown = sample;

/** Returns a fresh copy of the sample, parsed. */
export function sampleDocument(): ComplianceDocument {
  return parseDocument(structuredClone(sample));
}

/** The code of the per-rule refusal in `refusedSampleJson`. */
export const REFUSAL_CODE = "board::rules::checker_failed";

/**
 * Returns the sample with what a build writes when a report goes wrong: a
 * ninth column, legacy-cron, whose report the build refused as invalid; a
 * refusal of one rule in data-pipeline's report; and a rule whose checker
 * this build does not know, `regex`.
 */
export function refusedSampleJson(): unknown {
  const json: {
    data: {
      reports: { invalid_report: number };
      repositories: Record<string, unknown>[];
      rows: {
        id: string;
        checker: Record<string, unknown> | null;
        cells: Record<string, unknown>[];
      }[];
    };
  } = JSON.parse(JSON.stringify(sample));
  const data = json.data;
  const template = data.repositories.find((column) => column.name === "mobile-app");
  data.repositories.push({
    ...template,
    name: "legacy-cron",
    group: null,
    report: "invalid_report",
    code: "flightdeck::compliance::invalid_report",
    field: "rules[0].result",
    message: "unknown variant `sideways`",
  });
  data.reports.invalid_report += 1;
  for (const row of data.rows) {
    row.cells.push({
      repository: "legacy-cron",
      state: "not_readable",
      code: "flightdeck::compliance::invalid_report",
      result: null,
      facts: null,
      run: null,
      age_s: null,
    });
  }
  const fileSize = data.rows.find((row) => row.id === "/rules/python/file-size");
  const refused = fileSize?.cells.find((cell) => cell.repository === "data-pipeline");
  if (refused !== undefined) {
    refused.state = "not_readable";
    refused.code = REFUSAL_CODE;
    refused.result = "unreadable";
  }
  const pipeline = data.repositories.find((column) => column.name === "data-pipeline");
  if (pipeline !== undefined) {
    pipeline.refused = [
      {
        code: REFUSAL_CODE,
        rules: ["/rules/python/file-size"],
        message: "the checker could not read src/",
      },
    ];
  }
  const changelog = data.rows.find((row) => row.id === "/rules/docs/changelog-entry");
  if (changelog !== undefined) changelog.checker = { check: "regex" };
  return json;
}
