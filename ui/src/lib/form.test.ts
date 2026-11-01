import { describe, expect, it } from "vitest";
import { command, shelf } from "../../tests/fixtures/shelf";
import { draftFrom, emptyDraft, formOf, lineOf, missing, requestOf, setValue } from "./form";

const show = command(shelf, "books show");
const list = command(shelf, "books list");

function names(groups: ReturnType<typeof formOf>["required"]) {
  return groups.map((group) => group.choices.map((choice) => choice.name));
}

describe("formOf", () => {
  it("puts required fields first, with a switch between an argument and its one_of", () => {
    const form = formOf(show);

    expect(names(form.required)).toEqual([["<ID>", "--from"]]);
    expect(names(form.more)).toEqual([]);
  });

  it("puts flags with defaults and crumbkit's paging flags under more options", () => {
    const form = formOf(list);

    expect(names(form.required)).toEqual([]);
    expect(names(form.more)).toEqual([["--limit"], ["--all"]]);
  });

  it("leaves out crumbkit's own flags, which flightdeck sets itself", () => {
    const remove = formOf(command(shelf, "books remove"));
    const every = [...remove.required, ...remove.more].flatMap((group) =>
      group.choices.map((choice) => choice.name),
    );

    expect(every).toEqual(["<ID>"]);
  });
});

describe("requestOf", () => {
  it("sends structured values by the model's argument names", () => {
    let draft = emptyDraft(show);
    draft = setValue(draft, "<ID>", { many: ["2", "5"] });

    expect(requestOf(show, draft)).toEqual({
      path: ["books", "show"],
      values: { "<ID>": { many: ["2", "5"] } },
    });
  });

  it("sends only the chosen alternative of a one_of switch", () => {
    let draft = emptyDraft(show);
    draft = setValue(draft, "<ID>", { many: ["2"] });
    draft = setValue(draft, "--from", { one: "ids.txt" });
    draft = { ...draft, chosen: { "<ID>": "--from" } };

    expect(requestOf(show, draft).values).toEqual({ "--from": { one: "ids.txt" } });
  });

  it("sends a flag only when it is on and a value only when it is not empty", () => {
    let draft = emptyDraft(list);
    draft = setValue(draft, "--all", { flag: true });
    draft = setValue(draft, "--limit", { one: "" });

    expect(requestOf(list, draft).values).toEqual({ "--all": { flag: true } });
  });
});

describe("missing", () => {
  it("names a required field that has no value", () => {
    expect(missing(show, emptyDraft(show))).toEqual(["<ID>"]);
    expect(missing(show, setValue(emptyDraft(show), "<ID>", { many: ["2"] }))).toEqual([]);
  });
});

describe("draftFrom", () => {
  it("fills the form from a run's request and chooses the alternative it used", () => {
    const draft = draftFrom(show, { path: ["books", "show"], values: { "--from": { one: "a" } } });

    expect(draft.chosen).toEqual({ "<ID>": "--from" });
    expect(requestOf(show, draft).values).toEqual({ "--from": { one: "a" } });
  });
});

describe("lineOf", () => {
  it("writes the path, the positionals, then the options, quoted where needed", () => {
    const request = {
      path: ["books", "list"],
      values: { "--limit": { one: "2" }, "--all": { flag: true as const } },
    };

    expect(lineOf("shelf", list, request)).toBe("shelf books list --limit 2 --all");
    expect(
      lineOf("shelf", show, {
        path: ["books", "show"],
        values: { "<ID>": { many: ["2", "it's"] } },
      }),
    ).toBe("shelf books show 2 'it'\\''s'");
  });
});
