import { matrixRows } from "@crumbkit/flightdeck-headless/matrix";
import { describe, expect, it } from "vitest";
import { sampleDocument } from "../../../tests/fixtures/compliance/sample.ts";
import { filtersAddress, groupsOf, shownColumns, shownRows } from "./filters.ts";

const document = sampleDocument();
const names = (columns: readonly number[]) =>
  columns.map((index) => document.repositories[index]?.name);

describe("the Compliance page's tile and group filters", () => {
  it("lists each group once, in the order the repositories name them", () => {
    expect(groupsOf(document)).toEqual(["Services", "Apps", "Platform"]);
  });

  it("keeps every repository and row with no filters", () => {
    const filters = filtersAddress.parse("");
    const columns = shownColumns(document, filters);
    expect(columns).toHaveLength(document.repositories.length);
    expect(shownRows(matrixRows(document), filters, columns)).toHaveLength(
      matrixRows(document).length,
    );
  });

  it("keeps only the rows with a failing cell when the Failing tile is on", () => {
    const filters = filtersAddress.parse("?tile=failing");
    const rows = shownRows(matrixRows(document), filters, shownColumns(document, filters));
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) {
      expect(row.cells.some((cell) => cell.state === "failing")).toBe(true);
    }
  });

  it("keeps only the repositories without a report when the No report tile is on", () => {
    const filters = filtersAddress.parse("?tile=no_report");
    expect(names(shownColumns(document, filters))).toEqual(["mobile-app"]);
  });

  it("keeps only one group's repositories", () => {
    const filters = filtersAddress.parse("?group=Platform");
    expect(names(shownColumns(document, filters))).toEqual([
      "infra-terraform",
      "data-pipeline",
      "docs-site",
    ]);
  });

  it("keeps the rows with a failing cell among the group's repositories only", () => {
    const filters = filtersAddress.parse("?tile=failing&group=Apps");
    const columns = shownColumns(document, filters);
    const rows = shownRows(matrixRows(document), filters, columns);
    expect(rows.map((row) => row.name)).toEqual(["Agent context stays under 1,000 words"]);
  });
});
