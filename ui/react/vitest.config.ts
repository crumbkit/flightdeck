import { defineConfig } from "vitest/config";

// The hooks' tests run in jsdom with Testing Library (APP-6).
export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["./vitest.setup.ts"],
    restoreMocks: true,
    unstubGlobals: true,
    // A failed test is never retried (BASE-12).
    retry: 0,
  },
});
