import { describe, expect, it } from "vitest";
import fixture from "../../../tests/fixtures/compliance/compliance.json" with { type: "json" };
import schema from "../../../tests/golden/schemas/flightdeck-compliance-v1.json" with {
  type: "json",
};
import { CELL_STATES_WORST_FIRST, InvalidDocument, parseDocument } from "./document.ts";

describe("parseDocument", () => {
  it("parses the compliance build's fixture", () => {
    const document = parseDocument(fixture);
    expect(document.title).toBe("Compliance");
    expect(document.repositories.map((column) => column.name)).toEqual([
      "owner/app",
      "owner/api",
      "owner/ci",
      "owner/gone",
    ]);
    expect(document.rows).toHaveLength(9);
    expect(document.groups.map((group) => group.id)).toEqual([
      "/rules/groups/rust",
      "/rules/groups/server",
    ]);
  });

  it("refuses a document of another schema with invalid_document at the schema field", () => {
    const wrong = { ...fixture, schema: "flightdeck/compliance/v2" };
    expect(() => parseDocument(wrong)).toThrow(InvalidDocument);
    try {
      parseDocument(wrong);
    } catch (error) {
      expect(error).toBeInstanceOf(InvalidDocument);
      if (error instanceof InvalidDocument) {
        expect(error.code).toBe("invalid_document");
        expect(error.path).toBe("schema");
      }
    }
  });

  it("names the path of the first field that does not parse", () => {
    const rows = fixture.data.rows.map((row, index) =>
      index === 0 ? { ...row, section: "elsewhere" } : row,
    );
    const wrong = { ...fixture, data: { ...fixture.data, rows } };
    expect(() => parseDocument(wrong)).toThrow(/data\.rows\.0\.section/);
  });

  it("refuses text that is not a document at all", () => {
    expect(() => parseDocument("not a document")).toThrow(InvalidDocument);
  });

  it("parses a document whose failures follow the published failure shape", () => {
    const failure = {
      code: "compliance::report::missing",
      message: "owner/gone has no report",
      recovery: { kind: "hint", text: "Run the report job." },
    };
    expect(parseDocument({ ...fixture, failures: [failure] }).title).toBe("Compliance");
  });

  it("refuses a failure without a code at failures.0.code", () => {
    const failure = { message: "owner/gone has no report", recovery: { kind: "retry" } };
    expect(() => parseDocument({ ...fixture, failures: [failure] })).toThrow(/^failures\.0\.code/);
  });

  it("refuses a checker that is not an object or null at data.rows.0.checker", () => {
    const rows = fixture.data.rows.map((row, index) =>
      index === 0 ? { ...row, checker: 5 } : row,
    );
    expect(() => parseDocument({ ...fixture, data: { ...fixture.data, rows } })).toThrow(
      /^data\.rows\.0\.checker/,
    );
  });

  it("drops a field that the published schema does not name", () => {
    const document = parseDocument({ ...fixture, data: { ...fixture.data, later: 1 } });
    expect(Object.hasOwn(document, "later")).toBe(false);
  });
});

describe("CELL_STATES_WORST_FIRST", () => {
  it("lists the cell states in the order the published schema lists them", () => {
    const published = schema.$defs.CellState.oneOf.map((state) => state.const);
    expect(CELL_STATES_WORST_FIRST).toEqual(published);
  });
});
