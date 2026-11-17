import { readFile } from "node:fs/promises";
import { expect, type Page } from "@playwright/test";
import { expectNoAccessibilityViolations } from "./checks";
import { type Served, test } from "./real";

// The page's address holds the launch code until the page removes it, so
// no trace is kept of these tests.
test.use({ trace: "off" });

/** Opens the opener page and waits for the signed-in page. */
async function signIn(page: Page, served: Served): Promise<void> {
  await page.goto(served.opener);
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();
}

/** The fixture of the server's environment that the unit tests read. */
const fixture = new URL("../fixtures/shelf-environment.json", import.meta.url);

test("shelf without the built-in has no Environment button", async ({ page, served }) => {
  await signIn(page, served);
  const views = page.getByRole("navigation", { name: "Views" });

  await expect(views.getByRole("button", { name: "Usage" })).toBeVisible();
  await expect(views.getByRole("button", { name: "Environment" })).toHaveCount(0);
});

test.describe("a shelf that mounts self environment list", () => {
  test.use({ tool: "environment shelf" });

  test("the environment fixture matches the server's reading of the built-in", async ({
    page,
    served,
  }) => {
    const reply = page.waitForResponse((response) => response.url().endsWith("/api/model"));
    await page.goto(served.opener);
    const model = await (await reply).json();

    const kept: unknown = JSON.parse(await readFile(fixture, "utf8"));
    expect(model.tool.environment).toEqual(kept);
  });

  test("the Environment button opens the table, crumbkit's rows under their heading and a withheld value as dots", async ({
    page,
    served,
  }) => {
    await signIn(page, served);

    await page
      .getByRole("navigation", { name: "Views" })
      .getByRole("button", { name: "Environment" })
      .click();

    await expect(page.getByRole("heading", { level: 1, name: "Environment" })).toBeVisible();
    await expect(page.getByText("16 declared · 1 required unset · 1 invalid")).toBeVisible();
    await expect(page.getByText("Values are as they were when flightdeck started.")).toBeVisible();
    const table = page.getByRole("table", { name: "Environment variables" });
    await expect(
      table.getByRole("columnheader", { name: "Read by crumbkit in every tool" }),
    ).toBeVisible();
    await expect(table.getByRole("img", { name: "withheld" })).toHaveText("••••••••");
    await expect(page.getByText("shelf self environment list --full")).toBeVisible();
    await expectNoAccessibilityViolations(page);

    const port = table.getByRole("row", { name: /^SHELF_PORT\b/ });
    await expect(port.getByRole("cell").nth(0)).toHaveText("port");
    await expect(port.getByRole("cell").nth(1)).toHaveText("default 8080");
    const level = table.getByRole("row", { name: /^SHELF_LOG_LEVEL\b/ });
    await expect(level.getByRole("cell").nth(0)).toHaveText("enumeration");
    const token = table.getByRole("row", { name: /^SHELF_MIRROR_TOKEN\b/ });
    await expect(token.getByRole("cell").nth(1)).toHaveText(
      "optional locally, required in production",
    );
    await expect(token.getByRole("cell").nth(2)).toHaveText("secret · 1Password");
    const catalogue = table.getByRole("row", { name: /^SHELF_CATALOGUE_URL\b/ });
    await expect(catalogue.getByRole("cell").nth(1)).toHaveText("required");

    await table.getByRole("button", { name: "SHELF_LOG_LEVEL" }).click();

    await expect(table.getByText("Every step, for finding a fault.")).toBeVisible();
    await expectNoAccessibilityViolations(page);
  });

  test("a failure that names a variable links it, and the link opens the view on its row", async ({
    page,
    served,
  }) => {
    await signIn(page, served);
    await page
      .getByRole("navigation", { name: "Commands" })
      .getByRole("button", { name: /^books list/ })
      .click();
    await page.getByRole("form", { name: "Composer" }).getByRole("button", { name: /Run/ }).click();
    const card = page.getByRole("region", { name: /^Run 1: shelf books list/ });
    const link = card.getByRole("button", { name: "SHELF_CATALOGUE_URL" }).first();
    await expect(link).toBeVisible();
    await expectNoAccessibilityViolations(page);

    await link.click();

    const table = page.getByRole("table", { name: "Environment variables" });
    const name = table.getByRole("button", { name: "SHELF_CATALOGUE_URL" });
    await expect(name).toHaveAttribute("aria-expanded", "true");
    await expect(name).toBeFocused();
    await expect(table.getByText("The base URL of the catalogue that shelf reads.")).toBeVisible();
  });

  test.describe("on a phone", () => {
    test.use({ viewport: { width: 375, height: 812 } });

    test("the view lists each variable in place of the table", async ({ page, served }) => {
      await signIn(page, served);

      await page
        .getByRole("navigation", { name: "Views" })
        .getByRole("button", { name: "Environment" })
        .click();

      await expect(page.getByRole("table", { name: "Environment variables" })).toBeHidden();
      const crumbkits = page.getByRole("list", { name: "Read by crumbkit in every tool" });
      await expect(crumbkits.getByRole("button", { name: "TERM" })).toBeVisible();
      const declared = page.getByRole("list", { name: "Variables that shelf declares" });
      await expect(declared.getByText("port · default 8080 · setting")).toBeVisible();
      await expect(
        declared.getByText(
          "string · optional locally, required in production · secret · 1Password",
        ),
      ).toBeVisible();
      await expectNoAccessibilityViolations(page);
    });
  });
});
