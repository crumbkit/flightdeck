import { describe, expect, it } from "vitest";
import { shelf } from "../../tests/fixtures/shelf";
import { requestOfExample } from "./example";

describe("requestOfExample", () => {
  it("reads an example's positionals into the request", () => {
    expect(requestOfExample(shelf, "shelf books show 2 5")).toEqual({
      path: ["books", "show"],
      values: { "<ID>": ["2", "5"] },
    });
  });

  it("reads flags and options, and drops the flags flightdeck sets itself", () => {
    expect(requestOfExample(shelf, "shelf books list --all --json")).toEqual({
      path: ["books", "list"],
      values: { "--all": true },
    });
    expect(requestOfExample(shelf, "shelf books list --limit=2 --fields id,title")).toEqual({
      path: ["books", "list"],
      values: { "--limit": "2" },
    });
  });

  it("reads a quoted word as one value", () => {
    expect(requestOfExample(shelf, "shelf books show --from 'my ids.txt'")).toEqual({
      path: ["books", "show"],
      values: { "--from": "my ids.txt" },
    });
  });

  it("reads no request from a line that names another tool or an unknown argument", () => {
    expect(requestOfExample(shelf, "ledger books list")).toBeNull();
    expect(requestOfExample(shelf, "shelf books list --limti 2")).toBeNull();
    expect(requestOfExample(shelf, "shelf books")).toBeNull();
  });
});
