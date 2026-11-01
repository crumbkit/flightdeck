import { readFile, writeFile } from "node:fs/promises";
import { expect, type Page } from "@playwright/test";
import { expectNoAccessibilityViolations } from "./checks";
import { type Served, test } from "./real";

// The page's address holds the launch code until the page removes it, so
// no trace is kept of these tests. The column picker's test reads the
// copied line from the clipboard.
test.use({ trace: "off", permissions: ["clipboard-read", "clipboard-write"] });

/** Opens the opener page and waits for the signed-in page. */
async function signIn(page: Page, served: Served): Promise<void> {
  await page.goto(served.opener);
  await expect(page.getByRole("region", { name: "Welcome to shelf" })).toBeVisible();
}

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

test("an example chip appends a run card", async ({ page, served }) => {
  await signIn(page, served);

  await page.getByRole("button", { name: "Show books 2 and 5" }).click();

  const card = page.getByRole("region", { name: /^Run 1: shelf books show 2 5, exit 0/ });
  await expect(card).toBeVisible();
  await expect(card.getByRole("table")).toContainText("Middlemarch");
  await expectNoAccessibilityViolations(page);
});

test("books list shows crumbkit's footer, long columns and reading in words", async ({
  page,
  served,
}) => {
  await signIn(page, served);

  await page.getByRole("button", { name: "Print the first page of books" }).click();
  const card = page.getByRole("region", { name: /^Run 1: shelf books list/ });
  await expect(card.getByText("Showing 4 of 6 books")).toBeVisible();
  const headers = card.getByRole("columnheader");
  await expect(headers.filter({ hasText: "reading" })).toHaveCount(0);

  await card.getByRole("checkbox", { name: "Long columns" }).check();

  for (const name of ["form", "pages", "reading"]) {
    await expect(headers.filter({ hasText: name })).toHaveCount(1);
  }
  await expect(card.getByRole("cell", { name: "9 h 20 min PT9H20M" })).toBeVisible();
  await expectNoAccessibilityViolations(page);
});

test("the column picker's choice appears in the copied line as --fields", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  await page.getByRole("button", { name: "Print the first page of books" }).click();
  const card = page.getByRole("region", { name: /^Run 1: shelf books list/ });

  await card.getByText("Columns", { exact: true }).click();
  await card.getByRole("checkbox", { name: "author" }).uncheck();
  await card.getByRole("checkbox", { name: "year" }).uncheck();
  await card.getByRole("button", { name: "Copy line" }).click();

  await expect(card.getByRole("button", { name: "Copied" })).toBeVisible();
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied).toBe("shelf books list --fields id,title");
  await expect(card.getByRole("columnheader")).toHaveCount(3);
});

test("Edit fills the form from run 3, labels it and focuses its first field", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  await page.getByRole("button", { name: "Print the first page of books" }).click();
  await page.getByRole("button", { name: "Print every book" }).click();
  await page.getByRole("button", { name: "Show books 2 and 5" }).click();
  const third = page.getByRole("region", { name: /^Run 3: shelf books show 2 5/ });
  await expect(third).toBeVisible();
  // The chip filled the composer with books show 2 5, so clear it first.
  await page
    .getByRole("navigation", { name: "Commands" })
    .getByRole("button", { name: /books list/ })
    .click();

  await third.getByRole("button", { name: "Edit" }).click();

  const composer = page.getByRole("form", { name: "Composer" });
  await expect(composer.getByText("Editing run 3")).toBeVisible();
  await expect(composer.getByLabel(/The ids of the books to show/)).toBeFocused();
  await expect(composer.getByRole("heading")).toHaveText("books show");
  await expect(composer.getByRole("button", { name: "Remove 2" })).toBeVisible();
  await expect(composer.getByRole("button", { name: "Remove 5" })).toBeVisible();
  await expectNoAccessibilityViolations(page);
});

test("Run again appends a card and keeps the old one", async ({ page, served }) => {
  await signIn(page, served);
  await page.getByRole("button", { name: "Show books 2 and 5" }).click();
  const first = page.getByRole("region", { name: /^Run 1: shelf books show 2 5, exit 0/ });
  await expect(first).toBeVisible();

  await first.getByRole("button", { name: "Run again" }).click();

  await expect(
    page.getByRole("region", { name: /^Run 2: shelf books show 2 5, exit 0/ }),
  ).toBeVisible();
  await expect(first).toBeVisible();
});

test("the 2 cell of books list opens books show with 2 filled and runs nothing", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  await page.getByRole("button", { name: "Print the first page of books" }).click();
  const card = page.getByRole("region", { name: /^Run 1: shelf books list/ });
  const words = "Opens books show with id 2 · inferred from the books resource";
  const cell = card.getByRole("button", { name: words });
  await expect(cell).toHaveAttribute("title", words);

  await cell.click();

  const composer = page.getByRole("form", { name: "Composer" });
  await expect(composer.getByRole("heading")).toHaveText("books show");
  await expect(composer.getByRole("button", { name: "Remove 2" })).toBeVisible();
  await expect(page.getByRole("region", { name: /^Run / })).toHaveCount(1);
  await expectNoAccessibilityViolations(page);
});

test("the chooser of books show lists the six books of books list", async ({ page, served }) => {
  await signIn(page, served);

  await page
    .getByRole("navigation", { name: "Commands" })
    .getByRole("button", { name: /books show/ })
    .click();

  const composer = page.getByRole("form", { name: "Composer" });
  const choices = composer.getByRole("group", { name: "Choices" });
  await expect(choices.getByRole("checkbox")).toHaveCount(6);
  await expect(choices).toContainText("Frankenstein · Mary Shelley · 1818");
  await expect(composer.getByText("From shelf books list, column id · inferred")).toBeVisible();
  await expectNoAccessibilityViolations(page);
});
