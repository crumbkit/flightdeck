import { describe, expect, it } from "vitest";
import { hrefOf, placeOf } from "./place";

describe("placeOf", () => {
  it("reads no fragment, or one of another form, as the start", () => {
    expect(placeOf("")).toEqual({ view: "start" });
    expect(placeOf("#/")).toEqual({ view: "start" });
    expect(placeOf("#/elsewhere")).toEqual({ view: "start" });
  });

  it("reads the workspaces list and one workspace by its name", () => {
    expect(placeOf("#/workspaces")).toEqual({ view: "workspaces" });
    expect(placeOf("#/workspaces/reading-list")).toEqual({
      view: "workspace",
      name: "reading-list",
    });
  });

  it("decodes an escaped name and reads an empty one as the list", () => {
    expect(placeOf("#/workspaces/a%20b")).toEqual({ view: "workspace", name: "a b" });
    expect(placeOf("#/workspaces/")).toEqual({ view: "workspaces" });
    expect(placeOf("#/workspaces/%E0%A4%A")).toEqual({ view: "workspaces" });
  });
});

describe("hrefOf", () => {
  it("writes each place as the fragment that placeOf reads back", () => {
    for (const place of [
      { view: "start" } as const,
      { view: "workspaces" } as const,
      { view: "workspace", name: "a b/c" } as const,
    ]) {
      expect(placeOf(hrefOf(place))).toEqual(place);
    }
  });
});
