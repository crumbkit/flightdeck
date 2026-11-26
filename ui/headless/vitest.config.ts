import { defineConfig } from "vitest/config";

// The core's tests run in Node with no DOM, as the core itself does.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
    restoreMocks: true,
    unstubGlobals: true,
    // A failed test is never retried (BASE-12).
    retry: 0,
  },
});
