import { describe, expect, it } from "vitest";
import fixture from "../../../tests/fixtures/compliance/compliance.json" with { type: "json" };
import { InvalidDocument } from "./document.ts";
import { type ComplianceSource, readDocument, SourceFailed } from "./source.ts";

const signal = { aborted: false, reason: undefined };

describe("readDocument", () => {
  it("parses what the source reads", async () => {
    const source: ComplianceSource = { read: async () => fixture };
    expect((await readDocument(source, signal)).title).toBe("Compliance");
  });

  it("hands the source the signal it was given", async () => {
    const seen: unknown[] = [];
    const source: ComplianceSource = {
      read: async (given) => {
        seen.push(given);
        return fixture;
      },
    };
    await readDocument(source, signal);
    expect(seen).toEqual([signal]);
  });

  it("passes a failed read on as source_failed, with the error as its cause", async () => {
    const failure = new Error("network down");
    const source: ComplianceSource = {
      read: async () => {
        throw failure;
      },
    };
    const error = await readDocument(source, signal).catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(SourceFailed);
    if (error instanceof SourceFailed) {
      expect(error.code).toBe("source_failed");
      expect(error.cause).toBe(failure);
    }
  });

  it("refuses what is not a compliance document", async () => {
    const source: ComplianceSource = { read: async () => ({ schema: "other" }) };
    await expect(readDocument(source, signal)).rejects.toBeInstanceOf(InvalidDocument);
  });
});
