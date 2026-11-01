import { readFile, writeFile } from "node:fs/promises";
import { expect } from "@playwright/test";
import { expectNoAccessibilityViolations } from "./checks";
import { test } from "./real";

// The page's address holds the launch code until the page removes it, so
// no trace is kept of these tests.
test.use({ trace: "off" });

/** The fixture that the stubbed journeys and the unit tests read. */
const fixture = new URL("../fixtures/shelf-model.json", import.meta.url);

test("the opener page signs in and the page drops the launch code from its address", async ({
  page,
  served,
}) => {
  await page.goto(served.opener);

  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();
  expect(new URL(page.url()).hash).toBe("");
  await expectNoAccessibilityViolations(page);
});

test("the rail of crumbkit 0.1.0's shelf lists its commands under no heading", async ({
  page,
  served,
}) => {
  await page.goto(served.opener);
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();
  const rail = page.getByRole("navigation", { name: "Commands" });

  await expect(rail.getByRole("heading")).toHaveCount(0);
  await expect(rail.getByRole("button")).toHaveCount(9);
  await expect(rail.getByRole("button", { name: "books remove destructive" })).toBeVisible();
});

test("⌘K finds books show by the caption of its example", async ({ page, served }) => {
  await page.goto(served.opener);
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();

  await page.keyboard.press("ControlOrMeta+k");
  const palette = page.getByRole("dialog", { name: "Find a command" });
  await palette.getByRole("combobox", { name: "Command" }).fill("Show books 2 and 5");

  await expect(palette.getByRole("option").first()).toContainText("books show");
  await expectNoAccessibilityViolations(page);
  await page.keyboard.press("Enter");
  await expect(palette).toBeHidden();
  const composer = page.getByRole("form", { name: "Composer" });
  await expect(composer.getByRole("button", { name: "Remove 5" })).toBeVisible();
});

test("the model fixture matches the server's model of shelf", async ({ page, served }, info) => {
  const reply = page.waitForResponse((response) => response.url().endsWith("/api/model"));
  await page.goto(served.opener);
  const model: unknown = await (await reply).json();

  // One project writes the fixture, so two writers never race. `pnpm run
  // fix` formats it afterwards.
  if (process.env.FIXTURES === "overwrite" && info.project.name === "light") {
    await writeFile(fixture, `${JSON.stringify(model, null, 2)}\n`);
  }
  const kept: unknown = JSON.parse(await readFile(fixture, "utf8"));
  expect(model, "run FIXTURES=overwrite pnpm run acceptance, then pnpm run fix").toEqual(kept);
});
