import { describe, expect, it } from "vitest";
import { command, shelf } from "../../tests/fixtures/shelf";
import {
  draftFrom,
  emptyDraft,
  formOf,
  lineOf,
  lineOfDraft,
  longOf,
  missing,
  requestOf,
  setValue,
} from "./form";

const show = command(shelf, "books show");
const list = command(shelf, "books list");

function names(groups: ReturnType<typeof formOf>["required"]) {
  return groups.map((group) => group.choices.map((choice) => choice.name));
}

describe("formOf", () => {
  it("puts required fields first, with a switch between an argument and its one_of", () => {
    const form = formOf(show);

    expect(names(form.required)).toEqual([["<ID>", "--from"]]);
    expect(names(form.more)).toEqual([["--long"]]);
  });

  it("puts flags with defaults, crumbkit's paging flags and --long under more options", () => {
    const form = formOf(list);

    expect(names(form.required)).toEqual([]);
    expect(names(form.more)).toEqual([["--limit"], ["--all"], ["--long"]]);
  });

  it("leaves out crumbkit's own flags, which flightdeck sets itself, and --long when no column is long", () => {
    const summarize = formOf(command(shelf, "books summarize"));
    const every = [...summarize.required, ...summarize.more].flatMap((group) =>
      group.choices.map((choice) => choice.name),
    );

    expect(every).toEqual(["<ID>"]);
  });
});

describe("requestOf", () => {
  it("sends structured values by the model's argument names", () => {
    let draft = emptyDraft(show);
    draft = setValue(draft, "<ID>", ["2", "5"]);

    expect(requestOf(show, draft)).toEqual({
      path: ["books", "show"],
      values: { "<ID>": ["2", "5"] },
    });
  });

  it("sends only the chosen alternative of a one_of switch", () => {
    let draft = emptyDraft(show);
    draft = setValue(draft, "<ID>", ["2"]);
    draft = setValue(draft, "--from", "ids.txt");
    draft = { ...draft, chosen: { "<ID>": "--from" } };

    expect(requestOf(show, draft).values).toEqual({ "--from": "ids.txt" });
  });

  it("sends a flag only when it is on and a value only when it is not empty", () => {
    let draft = emptyDraft(list);
    draft = setValue(draft, "--all", true);
    draft = setValue(draft, "--limit", "");

    expect(requestOf(list, draft).values).toEqual({ "--all": true });
  });
});

describe("--long", () => {
  it("stays out of the run request and in the line, and asks for the table's long columns", () => {
    const draft = setValue(setValue(emptyDraft(show), "<ID>", ["2"]), "--long", true);

    expect(requestOf(show, draft).values).toEqual({ "<ID>": ["2"] });
    expect(lineOfDraft("shelf", show, draft)).toBe("shelf books show 2 --long");
    expect(longOf(draft)).toBe(true);
    expect(longOf(emptyDraft(show))).toBe(false);
  });
});

describe("missing", () => {
  it("names a required field that has no value", () => {
    expect(missing(show, emptyDraft(show))).toEqual(["<ID>"]);
    expect(missing(show, setValue(emptyDraft(show), "<ID>", ["2"]))).toEqual([]);
  });
});

describe("draftFrom", () => {
  it("fills the form from a run's request and chooses the alternative it used", () => {
    const draft = draftFrom(show, { path: ["books", "show"], values: { "--from": "a" } });

    expect(draft.chosen).toEqual({ "<ID>": "--from" });
    expect(requestOf(show, draft).values).toEqual({ "--from": "a" });
  });
});

describe("lineOf", () => {
  it("writes the path, the positionals, then the options, quoted where needed", () => {
    const request = {
      path: ["books", "list"],
      values: { "--limit": "2", "--all": true as const },
    };

    expect(lineOf("shelf", list, request)).toBe("shelf books list --limit 2 --all");
    expect(
      lineOf("shelf", show, {
        path: ["books", "show"],
        values: { "<ID>": ["2", "it's"] },
      }),
    ).toBe("shelf books show 2 'it'\\''s'");
  });
});
