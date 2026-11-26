import { describe, expect, it } from "vitest";
import { takeLaunchCode } from "./launch";

describe("takeLaunchCode", () => {
  it("reads the code from the fragment and removes the fragment from the address", () => {
    window.history.replaceState(null, "", "/#code=launch-code-for-test");

    expect(takeLaunchCode(window.location, window.history)).toBe("launch-code-for-test");
    expect(window.location.hash).toBe("");
    expect(window.location.href).not.toContain("launch-code-for-test");
  });

  it("keeps a fragment without a code, so a reload stays on its place", () => {
    window.history.replaceState(null, "", "/#/workspaces/reading-list");

    expect(takeLaunchCode(window.location, window.history)).toBeNull();
    expect(window.location.hash).toBe("#/workspaces/reading-list");
  });

  it("reads no code from an address without one", () => {
    window.history.replaceState(null, "", "/");

    expect(takeLaunchCode(window.location, window.history)).toBeNull();
  });
});
