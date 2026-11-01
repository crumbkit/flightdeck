import { describe, expect, it } from "vitest";
import { resultOf } from "./result";

const frankenstein = { id: 2, title: "Frankenstein" };

describe("resultOf", () => {
  it("draws a listing as a table with its total and whether it was cut", () => {
    const line = JSON.stringify({
      schema: "shelf/books-list/v1",
      data: [frankenstein, { id: 3, title: "Moby-Dick" }],
      total: 6,
      truncated: true,
    });

    expect(resultOf([line]).result).toEqual({
      kind: "table",
      rows: [frankenstein, { id: 3, title: "Moby-Dick" }],
      listing: { total: 6, truncated: true },
      failures: [],
    });
  });

  it("draws one row, or an object, as a record", () => {
    const one = JSON.stringify({ schema: "s", data: [frankenstein], failures: [] });
    const object = JSON.stringify({ schema: "s", data: { books: 6 }, failures: [] });

    expect(resultOf([one]).result).toMatchObject({ kind: "record", record: frankenstein });
    expect(resultOf([object]).result).toMatchObject({ kind: "record", record: { books: 6 } });
  });

  it("draws text as prose", () => {
    const line = JSON.stringify({ schema: "s", data: "Six books by six authors." });

    expect(resultOf([line]).result).toEqual({ kind: "prose", text: "Six books by six authors." });
  });

  it("keeps the failures of a batch beside its rows", () => {
    const line = JSON.stringify({
      schema: "s",
      data: [frankenstein],
      failures: [{ key: "9", index: 1, code: "shelf::books::not_found", message: "no book 9" }],
    });

    expect(resultOf([line]).result).toMatchObject({
      kind: "record",
      failures: [{ key: "9", code: "shelf::books::not_found", message: "no book 9" }],
    });
  });

  it("reads a stream's earlier lines as its rows", () => {
    const lines = [JSON.stringify({ n: 1 }), JSON.stringify({ n: 2 }), JSON.stringify({ n: 3 })];

    expect(resultOf(lines).result).toMatchObject({ kind: "table", rows: [{ n: 1 }, { n: 2 }] });
  });

  it("finds no result in no output or in output that is not JSON", () => {
    expect(resultOf([]).result).toEqual({ kind: "none" });
    expect(resultOf(["Showing 4 of 6 books."]).result).toEqual({ kind: "none" });
  });
});
