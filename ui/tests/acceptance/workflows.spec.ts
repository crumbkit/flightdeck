import { expect } from "@playwright/test";
import { developmentFlow, workflowsList } from "../fixtures/workflows";
import { expectNoAccessibilityViolations } from "./checks";
import { test } from "./real";

// The page's address holds the launch code until the page removes it, so
// no trace is kept of these tests.
test.use({ trace: "off", tool: "no tool" });

test("the rail's Workflows opens the fake board's development flow as a swimlane", async ({
  page,
  served,
}) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const list = page.waitForResponse((response) => response.url().endsWith("/api/workflows"));
  await page.goto(served.opener);
  expect(await (await list).json()).toEqual(workflowsList);
  const views = page.getByRole("navigation", { name: "Views" });
  const workflows = views.getByRole("link", { name: /^Workflows/ });
  await expect(workflows).toContainText("5");
  const shown = page.waitForResponse((response) =>
    response.url().endsWith("/api/workflows/development-flow"),
  );

  await workflows.click();

  expect(await (await shown).json()).toEqual(developmentFlow);
  await expect(page.getByRole("heading", { level: 1, name: "Development flow" })).toBeVisible();
  const graph = page.getByRole("region", { name: "Graph" });
  await expect(graph.getByRole("button", { name: /^Step: Build test-first/ })).toBeVisible();
  await expect(page.getByText("Not checked against the running flow")).toBeVisible();
  await expectNoAccessibilityViolations(page);

  await graph.getByRole("button", { name: /^Waits: Paused/ }).click();
  const panel = page.getByRole("complementary", { name: "Selected" });
  await expect(panel.getByRole("heading", { name: "Paused" })).toBeVisible();
  await expect(panel.getByRole("button", { name: "A paused change" })).toBeVisible();
  await expect(graph.getByRole("button", { name: /^Check: Plan names its task/ })).toHaveCSS(
    "opacity",
    "0.55",
  );
  await expectNoAccessibilityViolations(page);

  await page
    .getByRole("group", { name: "Workflow" })
    .getByRole("button", { name: "Tangled flow" })
    .click();
  await expect(page.getByRole("alert")).toContainText("The check found 1 problem");
  await expect(page.getByRole("region", { name: "Graph" })).toHaveCount(0);
  await expectNoAccessibilityViolations(page);
});

test("a change page's link selects the node that holds the change", async ({ page, served }) => {
  await page.goto(served.opener);
  await page
    .getByRole("navigation", { name: "Views" })
    .getByRole("link", { name: /^Workflows/ })
    .waitFor();

  await page.evaluate(() => {
    window.location.hash = "#/workflows/development-flow?selected=%2Fchanges%2Fsample-paused";
  });

  await expect(page.getByRole("button", { name: /^Waits: Paused/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
});

test("on a phone the graph is a run sheet with its rail behind Views", async ({ page, served }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(served.opener);
  await page.evaluate(() => {
    window.location.hash = "#/workflows/development-flow";
  });

  await expect(page.getByRole("heading", { level: 1, name: "Development flow" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Graph" })).toHaveCount(0);
  await expect(page.getByRole("checkbox", { name: /Hide checks/ })).toBeChecked();
  await expect(page.getByRole("navigation", { name: "Views" })).toBeHidden();
  const approval = page.getByRole("button", { name: /signs the plan/ });
  await approval.click();
  await expect(approval).toHaveAttribute("aria-expanded", "true");
  const height = await approval.evaluate((button) => button.getBoundingClientRect().height);
  expect(height).toBeGreaterThanOrEqual(44);
  await expectNoAccessibilityViolations(page);

  await page.getByRole("button", { name: "Views" }).click();
  await expect(page.getByRole("navigation", { name: "Views" })).toBeVisible();
  await expectNoAccessibilityViolations(page);
});
