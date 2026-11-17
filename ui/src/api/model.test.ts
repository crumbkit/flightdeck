import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { z } from "@/api/zod";
import { environmentShelf } from "../../tests/fixtures/shelf";
import { ModelReply, sourceKinds, typeNames } from "./model";

/** crumbkit-starmap's schema of `self environment list`, which the server's tests check. */
const schema = z
  .object({
    $defs: z.object({
      TypeName: z.object({ oneOf: z.array(z.object({ const: z.string() })) }),
      SourceWord: z.object({ oneOf: z.array(z.object({ const: z.string() })) }),
    }),
  })
  .parse(
    JSON.parse(
      // Vitest runs in ui/, and the schema is the server's test fixture.
      readFileSync(
        path.join(process.cwd(), "../tests/fixtures/starmap/self-environment-list-v1.json"),
        "utf8",
      ),
    ),
  );

/** Returns the words of the schema's closed list `name`. */
function closedList(name: "TypeName" | "SourceWord"): string[] {
  return schema.$defs[name].oneOf.map((each) => each.const);
}

describe("a variable's closed lists", () => {
  it("hold the words of crumbkit-starmap's schema", () => {
    expect([...typeNames]).toEqual(closedList("TypeName"));
    expect([...sourceKinds]).toEqual(closedList("SourceWord"));
  });

  it("read a word that a later crumbkit-starmap adds as other", () => {
    const [first] = environmentShelf.environment ?? [];
    const tool = {
      ...environmentShelf,
      environment: [
        { ...first, type_name: "duration", source_kind: { kind: "derived", bind: null } },
      ],
    };

    const reply = ModelReply.parse({ directory: null, read_only: false, tool });

    expect(reply.tool.environment?.[0]?.type_name).toBe("other");
    expect(reply.tool.environment?.[0]?.source_kind?.kind).toBe("other");
  });
});
