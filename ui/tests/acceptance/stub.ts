import AxeBuilder from "@axe-core/playwright";
import { test as base, expect, type Page } from "@playwright/test";
import { ModelReply } from "@/api/model";
import model from "../fixtures/shelf-model.json" with { type: "json" };
import runs from "../fixtures/shelf-runs.json" with { type: "json" };

/**
 * A stub of the server's API for the built page, which `vite preview`
 * serves. It answers the session exchange, the model and runs, the last
 * with shelf's real output for each request in `shelf-runs.json`, and
 * refuses any other run with the runner's failure document.
 *
 * The stub lets the page's journeys run before the server's run routes
 * exist. It misses the real server's checks and its content security
 * policy. Running the same tests against `flightdeck open shelf
 * --no-browser` replaces it.
 */
export async function openShelf(page: Page, reply: ModelReply = shelfReply): Promise<void> {
  const code = "stub-launch-code";
  await page.route("**/api/session", (route) =>
    route.request().postDataJSON()?.code === code
      ? route.fulfill({ json: { token: "stub-token" } })
      : route.fulfill({ status: 401, json: { title: "refused" } }),
  );
  await page.route("**/api/model", (route) => route.fulfill({ json: reply }));
  await page.route("**/api/runs", (route) => {
    const request = route.request().postDataJSON();
    const run = runs.find((each) => JSON.stringify(each.request) === JSON.stringify(request));
    if (run === undefined) {
      return route.fulfill({
        status: 422,
        contentType: "application/problem+json",
        body: JSON.stringify({
          code: "flightdeck::run::refused",
          cause: "the stub has no output for this request",
          fields: { path: request?.path?.join(" "), argument: null, reason: "stub" },
        }),
      });
    }
    const events = [
      { event: "started", run: "stub-run", line: run.line, started_at: "2026-11-02T09:00:00Z" },
      ...run.stdout.map((text) => ({ event: "line", text })),
      { event: "ended", outcome: { kind: "result" }, exit: run.exit, duration_ms: 41 },
    ];
    return route.fulfill({
      contentType: "application/x-ndjson",
      body: events.map((event) => `${JSON.stringify(event)}\n`).join(""),
    });
  });
  await page.goto(`/#code=${code}`);
  await expect(page.getByRole("region", { name: `Welcome to ${reply.tool.name}` })).toBeVisible();
}

/** shelf's model from crumbkit 0.1.0, which prints no goal headings. */
export const shelfReply: ModelReply = ModelReply.parse(model);

/**
 * Playwright's test, which fails any test whose page logs an error,
 * including a Content-Security-Policy violation (APP-7).
 */
export const test = base.extend<{ errors: string[] }>({
  errors: [
    async ({ page }, use) => {
      const errors: string[] = [];
      page.on("console", (message) => {
        if (message.type() === "error") {
          errors.push(message.text());
        }
      });
      await use(errors);
      expect(errors).toEqual([]);
    },
    { auto: true },
  ],
});

/** Checks the page with axe against WCAG 2.0, 2.1 and 2.2 A and AA (APP-8). */
export async function expectNoAccessibilityViolations(page: Page) {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(results.violations).toEqual([]);
}
