import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";

// jsdom does not implement scrolling, and the router scrolls to the top on
// each navigation. The stub keeps jsdom's "not implemented" message out of
// the test output; acceptance tests cover real scrolling (APP-7).
beforeEach(() => {
  vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
});

// Testing Library cleans up by itself only when test globals are on, and
// they are off, so each test unmounts what it rendered here.
afterEach(() => {
  cleanup();
});
