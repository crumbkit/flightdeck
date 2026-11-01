import { describe, expect, it, vi } from "vitest";
import { resultOf, streamReader } from "./result";

const frankenstein = { id: 2, title: "Frankenstein" };
const notFound = (key: string, index: number) => ({
  key,
  index,
  code: "shelf::books::not_found",
  message: `the catalogue has no book ${key}`,
});

describe("resultOf", () => {
  it("draws a listing as a table with its total and whether it was cut", () => {
    const line = JSON.stringify({
      schema: "shelf/books-list/v1",
      data: [frankenstein, { id: 3, title: "Moby-Dick" }],
      total: 6,
      truncated: true,
    });

    expect(resultOf([line], "document").result).toEqual({
      kind: "table",
      entries: [
        { kind: "row", row: frankenstein },
        { kind: "row", row: { id: 3, title: "Moby-Dick" } },
      ],
      listing: { total: 6, truncated: true },
    });
  });

  it("draws one row, or an object, as a record", () => {
    const one = JSON.stringify({ schema: "s", data: [frankenstein], failures: [] });
    const object = JSON.stringify({ schema: "s", data: { books: 6 }, failures: [] });

    expect(resultOf([one], "document").result).toEqual({
      kind: "record",
      entries: [{ kind: "row", row: frankenstein }],
    });
    expect(resultOf([object], "document").result).toEqual({
      kind: "record",
      entries: [{ kind: "row", row: { books: 6 } }],
    });
  });

  it("draws text as prose", () => {
    const line = JSON.stringify({ schema: "s", data: "Six books by six authors." });

    expect(resultOf([line], "document").result).toEqual({
      kind: "prose",
      text: "Six books by six authors.",
    });
  });

  it("places each failure of a batch at its index among the rows, in the order the keys were given", () => {
    const line = JSON.stringify({
      schema: "shelf/books-show/v1",
      data: [frankenstein],
      failures: [notFound("9", 2), notFound("7", 0)],
    });

    const { result } = resultOf([line], "document");

    expect(result.kind === "record" && result.entries.map((entry) => entry.kind)).toEqual([
      "failure",
      "row",
      "failure",
    ]);
    expect(result).toMatchObject({
      entries: [{ failure: { key: "7" } }, { row: frankenstein }, { failure: { key: "9" } }],
    });
  });

  it("reads each line of a stream as one row, and a failure line in its place", () => {
    const lines = [
      JSON.stringify({ schema: "reading/entries-stream/v1", data: { position: 1, title: "Emma" } }),
      JSON.stringify({
        schema: "reading/failure/v1",
        code: "reading::entries::unreadable",
        message: "line 2 is not UTF-8",
      }),
      JSON.stringify({
        schema: "reading/entries-stream/v1",
        data: { position: 3, title: "Ivanhoe" },
      }),
    ];

    expect(resultOf(lines, "stream").result).toMatchObject({
      kind: "table",
      entries: [
        { kind: "row", row: { title: "Emma" } },
        { kind: "failure", failure: { code: "reading::entries::unreadable" } },
        { kind: "row", row: { title: "Ivanhoe" } },
      ],
      listing: null,
    });
  });

  it("reads the next steps of a listing, and none from a document without them", () => {
    const step = {
      text: "takes up to 100 ids in one call.",
      commands: [{ call: "shelf books show <ID>..." }],
    };
    const listing = JSON.stringify({
      schema: "s",
      data: [],
      total: 0,
      truncated: false,
      next: [step],
    });
    const plain = JSON.stringify({ schema: "s", data: [] });

    expect(resultOf([listing], "document").next).toEqual([step]);
    expect(resultOf([plain], "document").next).toEqual([]);
  });

  it("finds no result in no output or in output that is not JSON", () => {
    expect(resultOf([], "document").result).toEqual({ kind: "none" });
    expect(resultOf(["Showing 4 of 6 books."], "document").result).toEqual({ kind: "none" });
  });
});

describe("streamReader", () => {
  it("parses each line of a growing stream once, however often it reads", () => {
    const item = (position: number) =>
      JSON.stringify({ schema: "reading/entries-stream/v1", data: { position } });
    const read = streamReader();
    const parse = vi.spyOn(JSON, "parse");

    read([item(1), item(2)]);
    const output = read([item(1), item(2), item(3)]);

    expect(parse).toHaveBeenCalledTimes(3);
    parse.mockRestore();
    expect(output.result).toMatchObject({
      kind: "table",
      entries: [{ row: { position: 1 } }, { row: { position: 2 } }, { row: { position: 3 } }],
    });
    expect(output.document).toHaveLength(3);
  });
});
