import { describe, expect, it } from "vitest";
import { refusedSpan } from "./refusal";

const usage = (fields: Record<string, unknown>) => ({ code: "shelf::cli::usage", fields });

/** Returns the part of `line` that the span covers. */
function marked(line: string, span: [number, number] | null): string | null {
  return span === null ? null : line.slice(span[0], span[1]);
}

describe("refusedSpan", () => {
  it("marks an option's word with its value, as the server writes it", () => {
    const line = "shelf books list --limit=101";

    expect(marked(line, refusedSpan(line, usage({ argument: "--limit", value: "101" })))).toBe(
      "--limit=101",
    );
  });

  it("marks a refused positional value by its value", () => {
    const line = "shelf books show 2 x7";

    expect(marked(line, refusedSpan(line, usage({ argument: "<ID>", value: "x7" })))).toBe("x7");
  });

  it("marks a flag that the line holds alone", () => {
    const line = "shelf books list --all";

    expect(marked(line, refusedSpan(line, usage({ argument: "--all" })))).toBe("--all");
  });

  it("marks nothing for a failure that is not a refusal, or a token the line lacks", () => {
    const line = "shelf books list --limit=101";

    expect(
      refusedSpan(line, { code: "shelf::books::not_found", fields: { argument: "--limit" } }),
    ).toBeNull();
    expect(refusedSpan(line, usage({ argument: "--from" }))).toBeNull();
    expect(refusedSpan(line, null)).toBeNull();
  });
});
