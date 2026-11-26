import { describe, expect, it } from "vitest";
import { UnsupportedSchema, zodFromJsonSchema } from "./zodFromJsonSchema.ts";

const target = {
  generator: "ui/scripts/schemas/zodFromJsonSchema.ts",
  source: "tests/golden/schemas/example.json",
  root: "Envelope",
  zodImport: "../../zod.ts",
};

function generate(schema: unknown): string {
  return zodFromJsonSchema(schema, target);
}

describe("zodFromJsonSchema", () => {
  it("starts the file with a comment that names the generator and the source and says it is generated", () => {
    const text = generate({ type: "string" });
    expect(text.startsWith("// This file is generated.")).toBe(true);
    expect(text).toContain("ui/scripts/schemas/zodFromJsonSchema.ts");
    expect(text).toContain("tests/golden/schemas/example.json");
    expect(text).toContain('import { z } from "../../zod.ts";');
    expect(text).toContain("export const EnvelopeSchema = z.string();");
  });

  it("turns a oneOf of string constants into a zod enum in the schema's order", () => {
    const text = generate({
      oneOf: [
        { type: "string", const: "failing" },
        { type: "string", const: "stale" },
        { type: "string", const: "absent" },
      ],
    });
    expect(text).toContain('export const EnvelopeSchema = z.enum(["failing", "stale", "absent"]);');
  });

  it("makes a property the schema does not require optional and a type that admits null nullable", () => {
    const text = generate({
      type: "object",
      properties: {
        age_s: { type: ["integer", "null"], minimum: 0 },
        dry_run: { const: true },
      },
      required: ["age_s"],
    });
    expect(text).toContain("age_s: z.number().int().nonnegative().nullable(),");
    expect(text).toContain("dry_run: z.literal(true).optional(),");
  });

  it("reads an anyOf of a reference and null as the referenced schema made nullable", () => {
    const text = generate({
      $defs: { Drift: { type: "object", properties: { other_set: { type: "boolean" } } } },
      anyOf: [{ $ref: "#/$defs/Drift" }, { type: "null" }],
    });
    expect(text).toContain("export const EnvelopeSchema = DriftSchema.nullable();");
  });

  it("writes each definition after the definitions it references", () => {
    const text = generate({
      $defs: {
        Row: {
          type: "object",
          properties: { cells: { type: "array", items: { $ref: "#/$defs/Cell" } } },
        },
        Cell: { type: "object", properties: { state: { $ref: "#/$defs/CellState" } } },
        CellState: { enum: ["failing", "current"] },
      },
      $ref: "#/$defs/Row",
    });
    const order = ["CellStateSchema =", "CellSchema =", "RowSchema =", "EnvelopeSchema ="].map(
      (name) => text.indexOf(name),
    );
    expect(order.every((at) => at >= 0)).toBe(true);
    expect([...order].sort((a, b) => a - b)).toEqual(order);
  });

  it("reads a map with a key pattern as a record whose keys must match it", () => {
    const text = generate({
      type: "object",
      additionalProperties: { type: "integer", minimum: 0 },
      propertyNames: { pattern: "^[a-z0-9_]+$" },
    });
    expect(text).toContain(
      'z.record(z.string().regex(new RegExp("^[a-z0-9_]+$")), z.number().int().nonnegative())',
    );
  });

  it("reads a reference back into a schema it sits inside as unknown", () => {
    const text = generate({
      type: "object",
      $anchor: "related",
      properties: { related: { type: "array", items: { $ref: "#related" } } },
    });
    expect(text).toContain("related: z.array(z.unknown()).optional(),");
  });

  it("refuses a keyword it does not translate and names where it is", () => {
    expect(() =>
      generate({ type: "object", properties: { name: { type: "string", maxLength: 4 } } }),
    ).toThrow(UnsupportedSchema);
    expect(() =>
      generate({ type: "object", properties: { name: { type: "string", maxLength: 4 } } }),
    ).toThrow("/properties/name: maxLength");
  });

  it("admits the annotations description, title and format and writes the description as a doc comment", () => {
    const text = generate({
      $defs: {
        StateCounts: {
          title: "StateCounts",
          description: "How many cells read each state.",
          type: "object",
          properties: { failing: { type: "integer", format: "uint", minimum: 0 } },
          required: ["failing"],
        },
      },
      $ref: "#/$defs/StateCounts",
    });
    expect(text).toContain(
      "/** How many cells read each state. */\nexport const StateCountsSchema",
    );
    expect(text).toContain("failing: z.number().int().nonnegative(),");
  });
});
