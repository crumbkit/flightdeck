import { expect, type Page } from "@playwright/test";
import type { ModelReply } from "@/api/model";
import { shelfReply } from "../fixtures/shelf";
import runs from "../fixtures/shelf-runs.json" with { type: "json" };

/**
 * A stub of the server's API for the built page, which `vite preview`
 * serves. It answers the session exchange, the model and runs, the last
 * with shelf's real output for each request in `shelf-runs.json`, and
 * refuses any other run with the runner's failure document.
 *
 * The stub serves the journeys that run a command, because the server has
 * no run routes yet, and the journey with goal headings, because the shelf
 * that the tests install prints none. It misses the real server's checks
 * and its content security policy. The journeys that need only the session
 * and the model run against the real server, in real.spec.ts.
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
          fields: { path: request?.path, argument: null, reason: "stub" },
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
