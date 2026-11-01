import { defineConfig, mergeConfig } from "vitest/config";
import vite from "./vite.config.ts";

// Unit and component tests run in jsdom (APP-6). Acceptance tests run in
// real browsers through Playwright (APP-7) and are left out here.
export default mergeConfig(
  vite,
  defineConfig({
    test: {
      environment: "jsdom",
      include: ["src/**/*.test.{ts,tsx}"],
      setupFiles: ["./vitest.setup.ts"],
      restoreMocks: true,
      unstubGlobals: true,
      // A failed test is never retried (BASE-12).
      retry: 0,
    },
  }),
);
