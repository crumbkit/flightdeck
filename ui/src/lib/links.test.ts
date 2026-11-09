import { describe, expect, it } from "vitest";
import type { Link, Tool } from "@/api/model";
import { command, shelf } from "../../tests/fixtures/shelf";
import { cellLinkOf, cellLinkWords, chooserFooter, chooserLinkOf, filled } from "./links";

/** Returns shelf with only the links that `keep` accepts. */
function withLinks(keep: (link: Link) => boolean): Tool {
  return { ...shelf, links: shelf.links.filter(keep) };
}

const path = (link: Link | null) =>
  link === null ? null : `${link.source.join(" ")} -> ${link.target.join(" ")} ${link.argument}`;

describe("cellLinkOf", () => {
  it("opens the first command that the model links the id column to", () => {
    expect(path(cellLinkOf(shelf, command(shelf, "books list")))).toBe(
      "books list -> books show <ID>",
    );
  });

  it("prefers a read-only target to a link listed before it", () => {
    const toRemove = (link: Link) => link.target.join(" ") === "books remove";
    const removeFirst = {
      ...shelf,
      links: [...shelf.links.filter(toRemove), ...shelf.links.filter((link) => !toRemove(link))],
    };

    expect(path(cellLinkOf(removeFirst, command(shelf, "books list")))).toBe(
      "books list -> books show <ID>",
    );
  });

  it("ignores a target that declares both hints when the server calls it destructive", () => {
    const bothHints = shelf.commands.map((each) =>
      each.path.join(" ") === "books remove"
        ? {
            ...each,
            class: "destructive" as const,
            effects: { ...each.effects, read_only: true, destructive: true },
          }
        : each,
    );
    const toRemove = (link: Link) => link.target.join(" ") === "books remove";
    const removeFirst = {
      ...shelf,
      commands: bothHints,
      links: [...shelf.links.filter(toRemove), ...shelf.links.filter((link) => !toRemove(link))],
    };

    expect(path(cellLinkOf(removeFirst, command(shelf, "books list")))).toBe(
      "books list -> books show <ID>",
    );
  });

  it("gives no link to a command whose rows have no id column", () => {
    expect(cellLinkOf(shelf, command(shelf, "commands list"))).toBeNull();
  });
});

describe("cellLinkWords", () => {
  it("says where the cell goes and that the link is inferred", () => {
    const link = cellLinkOf(shelf, command(shelf, "books list"));

    expect(link === null ? null : cellLinkWords(link, "2")).toBe(
      "Opens books show with id 2 · inferred from the books resource",
    );
  });
});

describe("chooserLinkOf", () => {
  it("lists the choices of books show from books list", () => {
    expect(path(chooserLinkOf(shelf, command(shelf, "books show"), "<ID>"))).toBe(
      "books list -> books show <ID>",
    );
  });

  it("skips a source that is not read-only", () => {
    const onlyRemove = withLinks((link) => link.source.join(" ") === "books remove");

    expect(chooserLinkOf(onlyRemove, command(shelf, "books show"), "<ID>")).toBeNull();
  });

  it("skips a source that needs a value of its own", () => {
    const onlyShow = withLinks((link) => link.source.join(" ") === "books show");

    expect(chooserLinkOf(onlyShow, command(shelf, "books summarize"), "<ID>")).toBeNull();
  });

  it("gives no chooser to an argument that no link fills", () => {
    expect(chooserLinkOf(shelf, command(shelf, "books show"), "--from")).toBeNull();
  });
});

describe("chooserFooter", () => {
  it("names the tool, the source and the column and says the link is inferred", () => {
    const link = chooserLinkOf(shelf, command(shelf, "books show"), "<ID>");

    expect(link === null ? null : chooserFooter(shelf, link)).toBe(
      "From shelf books list, column id · inferred",
    );
  });
});

describe("filled", () => {
  it("puts a value into an argument that repeats as a one-item list", () => {
    expect(filled(command(shelf, "books show"), "<ID>", "2")).toEqual({
      path: ["books", "show"],
      values: { "<ID>": ["2"] },
      chosen: {},
    });
  });
});
