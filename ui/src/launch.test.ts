import { describe, expect, it } from "vitest";
import { takeLaunchCode } from "./launch";

describe("takeLaunchCode", () => {
  it("reads the code from the fragment and removes the fragment from the address", () => {
    window.history.replaceState(null, "", "/#code=launch-code-for-test");

    expect(takeLaunchCode(window.location, window.history)).toBe("launch-code-for-test");
    expect(window.location.hash).toBe("");
    expect(window.location.href).not.toContain("launch-code-for-test");
  });

  it("reads no code from an address without one", () => {
    window.history.replaceState(null, "", "/");

    expect(takeLaunchCode(window.location, window.history)).toBeNull();
  });
});
