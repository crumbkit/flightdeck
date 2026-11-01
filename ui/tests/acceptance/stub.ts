import { expect, type Page } from "@playwright/test";
import type { ModelReply } from "@/api/model";
import { shelfReply } from "../fixtures/shelf";

/**
 * A stub of the server's session exchange and model for the built page,
 * which `vite preview` serves, with a model that the test chooses.
 *
 * The stub serves the journey with goal headings, because the shelf that
 * the tests install prints none. It misses the real server's checks, its
 * content security policy and its run routes. Moving the test tools to a
 * crumbkit whose shelf prints goal headings replaces it with the real
 * server, in real.spec.ts, where every other journey runs.
 */
export async function openShelf(page: Page, reply: ModelReply = shelfReply): Promise<void> {
  const code = "stub-launch-code";
  await page.route("**/api/session", (route) =>
    route.request().postDataJSON()?.code === code
      ? route.fulfill({ json: { token: "stub-token" } })
      : route.fulfill({ status: 401, json: { title: "refused" } }),
  );
  await page.route("**/api/model", (route) => route.fulfill({ json: reply }));
  await page.goto(`/#code=${code}`);
  await expect(page.getByRole("region", { name: `Welcome to ${reply.tool.name}` })).toBeVisible();
}
