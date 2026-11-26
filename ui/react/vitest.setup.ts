import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// Testing Library cleans up by itself only when test globals are on, and
// they are off, so each test unmounts what it rendered here.
afterEach(() => {
  cleanup();
});
