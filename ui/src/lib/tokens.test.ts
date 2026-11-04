import { describe, expect, it } from "vitest";
import { tokenize } from "./tokens";

function values(text: string): string[] {
  return tokenize(text).tokens.map((token) => token.value);
}

describe("tokenize", () => {
  it("splits a line at runs of spaces and keeps where each word starts and ends", () => {
    expect(tokenize("books  list --all").tokens).toEqual([
      { value: "books", start: 0, end: 5, quoted: false },
      { value: "list", start: 7, end: 11, quoted: false },
      { value: "--all", start: 12, end: 17, quoted: false },
    ]);
  });

  it("keeps everything between single quotes as written, backslashes too", () => {
    expect(values("show 'my ids\\n.txt'")).toEqual(["show", "my ids\\n.txt"]);
    expect(tokenize("'a b'").tokens[0]).toEqual({ value: "a b", start: 0, end: 5, quoted: true });
  });

  it("lets a backslash in double quotes escape only a quote, a backslash, a dollar and a backtick", () => {
    expect(values('"say \\"hi\\" \\\\ \\$HOME \\n"')).toEqual(['say "hi" \\ $HOME \\n']);
  });

  it("lets a backslash outside quotes keep the next character, a space included", () => {
    expect(values("my\\ ids.txt \\'x")).toEqual(["my ids.txt", "'x"]);
  });

  it("joins quoted and plain parts that touch into one word, and reads empty quotes as an empty word", () => {
    expect(values("a'b c'd \"\" ''")).toEqual(["ab cd", "", ""]);
  });

  it("names the place of a quote that is not closed and ends the last word at the end of the line", () => {
    const tokenized = tokenize("show --from 'my ids");

    expect(tokenized.open).toBe(12);
    expect(tokenized.tokens.at(-1)).toEqual({ value: "my ids", start: 12, end: 19, quoted: true });
    expect(tokenize("show 'a'").open).toBeNull();
  });

  it("keeps a backslash at the end of the line as a backslash", () => {
    expect(values("a\\")).toEqual(["a\\"]);
  });
});
