import { describe, expect, it } from "vitest";
import { joinSearch, parsePlace, placeHash, sharedSource } from "./address.ts";

describe("the site's address", () => {
  it("reads an empty hash as the Compliance page", () => {
    expect(parsePlace("")).toEqual({ route: { page: "compliance" }, search: "" });
  });

  it("reads the Compliance page's filters from its query", () => {
    expect(parsePlace("#/?show=not_current")).toEqual({
      route: { page: "compliance" },
      search: "?show=not_current",
    });
  });

  it("reads a repository's name back from the hash it writes", () => {
    const hash = placeHash({ page: "repository", name: "owner/app" }, "?rule=%2Frules%2Fa");
    expect(hash).toBe("#/repositories/owner%2Fapp?rule=%2Frules%2Fa");
    expect(parsePlace(hash)).toEqual({
      route: { page: "repository", name: "owner/app" },
      search: "?rule=%2Frules%2Fa",
    });
  });

  it("reads the Rules page and its filters", () => {
    expect(parsePlace("#/rules?kind=standing")).toEqual({
      route: { page: "rules" },
      search: "?kind=standing",
    });
  });

  it("reads a hash it does not know as the Compliance page with no filters", () => {
    expect(parsePlace("#failing")).toEqual({ route: { page: "compliance" }, search: "" });
    expect(parsePlace("#/repositories/%E0%A4%A")).toEqual({
      route: { page: "compliance" },
      search: "",
    });
  });

  it("joins the queries of two sets of filters", () => {
    expect(joinSearch("?show=not_current", "", "?group=Platform")).toBe(
      "?show=not_current&group=Platform",
    );
    expect(joinSearch("", "")).toBe("");
  });

  it("keeps the page's other filters when one set changes", () => {
    const moves: string[] = [];
    const source = sharedSource("?show=every&group=Platform", "?group=Platform", (search) =>
      moves.push(search),
    );
    source.navigate("?show=not_current");
    expect(moves).toEqual(["?show=not_current&group=Platform"]);
  });
});
