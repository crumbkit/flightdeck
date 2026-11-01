import { describe, expect, it } from "vitest";
import { command, shelf } from "../../tests/fixtures/shelf";
import { columnsOf, copiedLine, footerOf, pageSizesOf, withPageSize } from "./table";

const list = command(shelf, "books list");
const book = {
  id: 2,
  title: "Frankenstein",
  author: "Mary Shelley",
  year: 1818,
  form: "novel",
  pages: 280,
  reading: "PT9H20M",
};

function names(view: Parameters<typeof columnsOf>[2]) {
  return columnsOf(list, [book], view).map((column) => column.name);
}

describe("columnsOf", () => {
  it("shows the compact columns with their descriptions by default", () => {
    const columns = columnsOf(list, [book], { long: false, fields: null });

    expect(columns.map((column) => column.name)).toEqual(["id", "title", "author", "year"]);
    expect(columns[0]?.description).toBe("The book's id, unique within the catalogue.");
  });

  it("adds form, pages and reading for long columns", () => {
    expect(names({ long: true, fields: null })).toEqual([
      "id",
      "title",
      "author",
      "year",
      "form",
      "pages",
      "reading",
    ]);
  });

  it("shows exactly the picked fields in the picked order", () => {
    expect(names({ long: true, fields: ["title", "id"] })).toEqual(["title", "id"]);
  });

  it("takes the columns from the first row when the command declares none", () => {
    const bare = { ...list, result: { ...list.result, columns: [] } };

    expect(columnsOf(bare, [{ a: 1, b: 2 }], { long: false, fields: null })).toEqual([
      { name: "a", long: false, description: null },
      { name: "b", long: false, description: null },
    ]);
  });
});

describe("footerOf", () => {
  it("words a cut listing as crumbkit does", () => {
    expect(footerOf(list, 4, { total: 6, truncated: true })).toBe("Showing 4 of 6 books");
    expect(footerOf(list, 4, { total: null, truncated: true })).toBe(
      "Showing 4 books, and more match",
    );
  });

  it("words nothing for a listing that shows every row", () => {
    expect(footerOf(list, 6, { total: 6, truncated: false })).toBeNull();
  });
});

describe("copiedLine", () => {
  it("adds the picked columns as --fields, or --long for long columns", () => {
    expect(copiedLine("shelf books list", { long: false, fields: ["id", "title"] })).toBe(
      "shelf books list --fields id,title",
    );
    expect(copiedLine("shelf books list", { long: true, fields: null })).toBe(
      "shelf books list --long",
    );
    expect(copiedLine("shelf books list", { long: false, fields: null })).toBe("shelf books list");
  });
});

describe("page sizes", () => {
  it("offers the default, 25, 100 and All for a command that pages", () => {
    expect(pageSizesOf(list)).toEqual(["default", "25", "100", "all"]);
    expect(pageSizesOf(command(shelf, "books show"))).toEqual([]);
  });

  it("replaces the request's paging flags with the chosen size", () => {
    const request = { path: ["books", "list"], values: { "--limit": "2" } };

    expect(withPageSize(request, "all").values).toEqual({ "--all": true });
    expect(withPageSize(request, "25").values).toEqual({ "--limit": "25" });
    expect(withPageSize(request, "default").values).toEqual({});
  });
});
