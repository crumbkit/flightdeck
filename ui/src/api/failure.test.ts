import { describe, expect, it } from "vitest";
import { fieldErrorOf } from "./failure";

const refusal = (fields: Record<string, unknown>) => ({
  code: "flightdeck::run::refused",
  cause: "the cause",
  fields,
});

describe("fieldErrorOf", () => {
  it("marks the refused argument with the refusal's cause", () => {
    const problem = refusal({ path: ["books", "list"], argument: "--limit", reason: "arity" });

    expect(fieldErrorOf(problem, "the cause")).toEqual({ name: "--limit", words: "the cause" });
  });

  it("marks the earlier argument that needs a value first", () => {
    const problem = refusal({ argument: "<TO>", reason: "skipped", earlier: "<FROM>" });

    expect(fieldErrorOf(problem, "the cause")).toEqual({ name: "<FROM>", words: "the cause" });
  });

  it("names the position of the refused value in a list", () => {
    const problem = refusal({ argument: "<ID>", reason: "control", index: 1 });

    expect(fieldErrorOf(problem, "the cause")).toEqual({
      name: "<ID>",
      words: "Value 2: the cause",
    });
  });

  it("marks nothing for a refusal that names no argument", () => {
    expect(fieldErrorOf(refusal({ path: ["books"], argument: null }), "x")).toBeNull();
    expect(fieldErrorOf(null, "x")).toBeNull();
  });
});
