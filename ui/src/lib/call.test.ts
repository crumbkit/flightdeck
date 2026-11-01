import { describe, expect, it } from "vitest";
import { shelf } from "../../tests/fixtures/shelf";
import { callLabel, requestOfCall } from "./call";

describe("requestOfCall", () => {
  it("fills the composer from a recovery's call, without the flags flightdeck sets itself", () => {
    expect(requestOfCall(shelf, "shelf books list --all --json", null)).toEqual({
      path: ["books", "list"],
      values: { "--all": true },
    });
  });

  it("leaves a placeholder's field empty", () => {
    expect(requestOfCall(shelf, "shelf books show <ID>...", null)).toEqual({
      path: ["books", "show"],
      values: {},
    });
    expect(requestOfCall(shelf, "shelf books list --limit <N>", null)).toEqual({
      path: ["books", "list"],
      values: {},
    });
  });

  it("puts every value of the failure's field in the placeholder's place", () => {
    expect(requestOfCall(shelf, "shelf books show <ID>...", [7, 9])).toEqual({
      path: ["books", "show"],
      values: { "<ID>": ["7", "9"] },
    });
  });
});

describe("callLabel", () => {
  it("names a call without the tool's name, with an ellipsis for a repeated placeholder", () => {
    expect(callLabel(shelf, "shelf books show <ID>...")).toBe("books show <ID>…");
    expect(callLabel(shelf, "shelf books list")).toBe("books list");
  });
});
