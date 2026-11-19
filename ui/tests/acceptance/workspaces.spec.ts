import { expect } from "@playwright/test";
import { readingList, workspacesList } from "../fixtures/workspaces";
import { expectNoAccessibilityViolations } from "./checks";
import { test } from "./real";

// The page's address holds the launch code until the page removes it, so
// no trace is kept of these tests.
test.use({ trace: "off" });

test.describe("flightdeck without a tool", () => {
  test.use({ tool: "no tool" });

  test("the page lists the fake mxv's workspaces, as the unit tests' fixture holds them", async ({
    page,
    served,
  }) => {
    const reply = page.waitForResponse((response) => response.url().endsWith("/api/workspaces"));
    await page.goto(served.opener);

    expect(await (await reply).json()).toEqual(workspacesList);
    await expect(
      page.getByRole("heading", { level: 1, name: "Workspaces on local" }),
    ).toBeVisible();
    await expect(page.getByText("1 running")).toBeVisible();
    const reading = page.getByRole("link", { name: /^reading-list/ });
    await expect(reading.getByText("2 of 3 running")).toBeVisible();
    await expect(reading.getByText("web.reading-list.mxv.localhost")).toBeVisible();
    const broken = page.getByRole("link", { name: /^broken-status/ });
    await expect(broken.getByText("units unknown · mxv::manifest::invalid")).toBeVisible();
    const views = page.getByRole("navigation", { name: "Views" });
    await expect(views.getByRole("link", { name: "Commands" })).toHaveCount(0);
    await expectNoAccessibilityViolations(page);
  });

  test("a card opens its workspace, and the breadcrumb comes back to the list", async ({
    page,
    served,
  }) => {
    await page.goto(served.opener);
    const reply = page.waitForResponse((response) =>
      response.url().endsWith("/api/workspaces/reading-list"),
    );

    await page.getByRole("link", { name: /^reading-list/ }).click();

    expect(await (await reply).json()).toEqual(readingList);
    await expect(page.getByRole("heading", { level: 1, name: "reading-list" })).toBeVisible();
    const units = page.getByRole("table", { name: "Units" });
    await expect(units.getByRole("row", { name: /cache/ })).toContainText("exited with 137");
    await expect(units.getByRole("link", { name: "route" })).toHaveAttribute(
      "href",
      "https://web.reading-list.mxv.localhost",
    );
    await expect(page.getByText("2 uncommitted changes")).toBeVisible();
    await expectNoAccessibilityViolations(page);

    await page.getByRole("navigation", { name: "Breadcrumb" }).getByRole("link").click();
    await expect(
      page.getByRole("heading", { level: 1, name: "Workspaces on local" }),
    ).toBeVisible();

    await page.goBack();
    await expect(page.getByRole("heading", { level: 1, name: "reading-list" })).toBeVisible();

    await page.reload();
    await expect(page.getByRole("heading", { level: 1, name: "reading-list" })).toBeVisible();
  });
});

test("the tool's page opens the workspaces, and the rail's Commands comes back to it", async ({
  page,
  served,
}) => {
  await page.goto(served.opener);
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();

  await page.getByRole("button", { name: "Workspaces", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: "Workspaces on local" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeHidden();
  await expectNoAccessibilityViolations(page);

  await page
    .getByRole("navigation", { name: "Views" })
    .getByRole("link", { name: "Commands" })
    .click();
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();
});
