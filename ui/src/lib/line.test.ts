import { describe, expect, it } from "vitest";
import { command, shelf } from "../../tests/fixtures/shelf";
import { draftFrom } from "./form";
import { notesOf, parseLine, stripPrompt, textOf } from "./line";

/** Parses `text` as the line reads it after the person leaves it. */
function parsed(text: string) {
  return parseLine(shelf, text, null);
}

describe("parseLine", () => {
  it("applies nothing while the caret is in the word being typed, and applies it once a space follows", () => {
    const typing = parseLine(shelf, "books list --li", 15);
    const typed = parseLine(shelf, "books list --limit 1 ", 21);

    expect(typing.pending).toBe(true);
    expect(typing.problem).toBeNull();
    expect(parseLine(shelf, "books list --limit 1", 20).pending).toBe(true);
    expect(typed.pending).toBe(false);
    expect(typed.command?.path).toEqual(["books", "list"]);
    expect(typed.values).toEqual({ "--limit": "1" });
  });

  it("reads an option's value after an equals sign or as the next word", () => {
    expect(parsed("books list --limit=10").values).toEqual({ "--limit": "10" });
    expect(parsed("books list --limit 10").values).toEqual({ "--limit": "10" });
  });

  it("refuses an unknown option in crumbkit's words and offers the similar one as a fix", () => {
    const { problem } = parsed("books list --limti 10");

    expect(problem).toEqual({
      start: 11,
      end: 18,
      words: "unexpected argument '--limti' found; a similar argument exists: '--limit'",
      fix: { start: 11, end: 18, text: "--limit" },
    });
  });

  it("refuses an unknown command in crumbkit's words and offers the similar one as a fix", () => {
    const { problem } = parsed("book list");

    expect(problem?.words).toBe(
      "unrecognized subcommand 'book'; a similar subcommand exists: 'books'",
    );
    expect(problem?.fix).toEqual({ start: 0, end: 4, text: "books" });
    expect(parsed("zebra").problem).toMatchObject({
      words: "unrecognized subcommand 'zebra'",
      fix: null,
    });
  });

  it("names the column of a quote that is not closed once the person leaves the line", () => {
    expect(parsed("books show --from 'my ids").problem?.words).toBe(
      "the quote at column 19 is not closed",
    );
    expect(parseLine(shelf, "books show --from 'my ids", 25).problem).toBeNull();
  });

  it("asks for the rest of a command path or an option's value only once the person leaves the line", () => {
    expect(parseLine(shelf, "books ", 6).pending).toBe(true);
    expect(parsed("books").problem?.words).toBe(
      "the command `shelf books` needs a subcommand: list, show, summarize, remove",
    );
    expect(parseLine(shelf, "books list --limit ", 19).pending).toBe(true);
    expect(parsed("books list --limit").problem?.words).toBe(
      "a value is required for '--limit <N>' but none was supplied",
    );
  });

  it("refuses a value that no argument takes, a value for a flag, a repeated option and both sides of a switch", () => {
    expect(parsed("books list extra").problem?.words).toBe("unexpected argument 'extra' found");
    expect(parsed("books list --all=x").problem?.words).toBe(
      "unexpected value 'x' for '--all' found; no more were expected",
    );
    expect(parsed("books list --limit 2 --limit 3").problem?.words).toBe(
      "the argument '--limit <N>' cannot be used multiple times",
    );
    expect(parsed("books show 2 --from x").problem?.words).toBe(
      "the argument '<ID>...' cannot be used with '--from <FILE>'",
    );
  });

  it("reads every word after -- as a positional value", () => {
    expect(parsed("books show -- -2 5").values).toEqual({ "<ID>": ["-2", "5"] });
  });

  it("fills the ids and the long columns from a line pasted from a terminal", () => {
    const pasted = parsed(stripPrompt(shelf, "$ shelf books show 2 5 --long"));

    expect(pasted.command?.path).toEqual(["books", "show"]);
    expect(pasted.values).toEqual({ "<ID>": ["2", "5"], "--long": true });
  });

  it("takes out the flags flightdeck adds itself and says so", () => {
    const line = parsed("books list --json --full --no-input --all");

    expect(line.values).toEqual({ "--all": true });
    expect(notesOf(line.dropped)).toEqual([
      "flightdeck adds --json, --full and --no-input itself, so they were taken out of the line.",
    ]);
  });

  it("takes out --yes, so a command that changes data still asks before it runs", () => {
    const line = parsed("books remove 2 --yes");

    expect(line.values).toEqual({ "<ID>": ["2"] });
    expect(notesOf(line.dropped)).toEqual([
      "flightdeck asks before it runs a command that changes data, so --yes was taken out of the line.",
    ]);
  });

  it("marks each word as part of the path, a flag, an option's value or a positional value", () => {
    const kinds = parsed("books show 2 --from 'a b'").marks.map((mark) => [mark.kind, mark.quoted]);

    expect(kinds).toEqual([
      ["path", false],
      ["path", false],
      ["positional", false],
      ["flag", false],
      ["value", true],
    ]);
  });
});

describe("stripPrompt", () => {
  it("drops a leading $ and the tool's name, and keeps a line without them", () => {
    expect(stripPrompt(shelf, "$ shelf books list")).toBe("books list");
    expect(stripPrompt(shelf, "shelf books list")).toBe("books list");
    expect(stripPrompt(shelf, "books list")).toBe("books list");
  });

  it("keeps the tool's name until a space follows it, so typing the name leaves it in place", () => {
    expect(stripPrompt(shelf, "shelf")).toBe("shelf");
    expect(stripPrompt(shelf, "$ shelf")).toBe("shelf");
    expect(stripPrompt(shelf, "shelf ")).toBe("");
  });
});

describe("the line and the form", () => {
  const examples = shelf.commands.flatMap((each) =>
    each.examples.map((example) => ({ path: each.path.join(" "), line: example.command })),
  );

  it.each(examples)("give back the example $line from the form it fills", ({ path, line }) => {
    const first = parsed(stripPrompt(shelf, line));
    const owner = command(shelf, path);
    const draft = draftFrom(owner, { path: owner.path, values: first.values });
    const text = textOf(shelf, owner, draft);
    const again = parsed(text);
    const kept = line
      .split(" ")
      .filter((word) => !first.dropped.includes(word))
      .join(" ");

    expect(first.problem).toBeNull();
    expect(first.command).toBe(owner);
    expect(`shelf ${text}`).toBe(kept);
    expect(again.values).toEqual(first.values);
  });
});
