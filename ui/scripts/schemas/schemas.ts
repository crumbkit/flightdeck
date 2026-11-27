// Writes every generated zod module from the JSON Schema it is generated from
// (BASE-21). `pnpm run schemas` runs this and then fails when git shows that a
// generated file differs from the committed one.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { zodFromJsonSchema } from "./zodFromJsonSchema.ts";

const repository = join(import.meta.dirname, "..", "..", "..");

const targets = [
  {
    // The golden output of `flightdeck schemas show flightdeck/compliance/v1`,
    // which the Rust tests keep equal to what the binary prints.
    source: "tests/golden/schemas/flightdeck-compliance-v1.json",
    output: "ui/headless/src/api/generated/compliance.ts",
    root: "ComplianceEnvelope",
    zodImport: "../../zod.ts",
  },
];

for (const { source, output, root, zodImport } of targets) {
  const schema: unknown = JSON.parse(readFileSync(join(repository, source), "utf8"));
  const text = zodFromJsonSchema(schema, {
    generator: "ui/scripts/schemas/zodFromJsonSchema.ts",
    source,
    root,
    zodImport,
  });
  const path = join(repository, output);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, text);
}
