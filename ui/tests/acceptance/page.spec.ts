import { expect } from "@playwright/test";
import { shelfReply } from "../fixtures/shelf";
import { expectNoAccessibilityViolations, test } from "./checks";
import { openShelf } from "./stub";

// The column picker's test reads the copied line from the clipboard.
test.use({ permissions: ["clipboard-read", "clipboard-write"] });

const goals = [
  {
    title: "Browse the catalogue",
    commands: [
      ["books", "list"],
      ["books", "show"],
      ["books", "summarize"],
      ["books", "remove"],
    ],
  },
  {
    title: "Explore the commands",
    commands: [
      ["commands", "list"],
      ["schemas", "list"],
      ["schemas", "show"],
    ],
  },
  {
    title: "Review how shelf is used",
    commands: [
      ["self", "usage"],
      ["self", "figures", "list"],
    ],
  },
];

test("the rail shows the three goal headings of a newer shelf in help order", async ({ page }) => {
  await openShelf(page, {
    ...shelfReply,
    tool: { ...shelfReply.tool, headings: goals },
  });
  const rail = page.getByRole("navigation", { name: "Commands" });

  await expect(rail.getByRole("heading")).toHaveText([
    "Browse the catalogue",
    "Explore the commands",
    "Review how shelf is used",
  ]);
  await expectNoAccessibilityViolations(page);
});

test("an example chip appends a run card", async ({ page }) => {
  await openShelf(page);

  await page.getByRole("button", { name: "Show books 2 and 5" }).click();

  const card = page.getByRole("region", { name: /^Run 1: shelf books show 2 5, exit 0/ });
  await expect(card).toBeVisible();
  await expect(card.getByRole("table")).toContainText("Middlemarch");
  await expectNoAccessibilityViolations(page);
});

test("books list shows crumbkit's footer, long columns and reading in words", async ({ page }) => {
  await openShelf(page);

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

test("the column picker's choice appears in the copied line as --fields", async ({ page }) => {
  await openShelf(page);
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

test("Edit fills the form from run 3 and labels it", async ({ page }) => {
  await openShelf(page);
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
  await expect(composer.getByRole("heading")).toHaveText("books show");
  await expect(composer.getByRole("button", { name: "Remove 2" })).toBeVisible();
  await expect(composer.getByRole("button", { name: "Remove 5" })).toBeVisible();
  await expectNoAccessibilityViolations(page);
});

test("Run again appends a card and keeps the old one", async ({ page }) => {
  await openShelf(page);
  await page.getByRole("button", { name: "Show books 2 and 5" }).click();
  const first = page.getByRole("region", { name: /^Run 1: shelf books show 2 5, exit 0/ });
  await expect(first).toBeVisible();

  await first.getByRole("button", { name: "Run again" }).click();

  await expect(
    page.getByRole("region", { name: /^Run 2: shelf books show 2 5, exit 0/ }),
  ).toBeVisible();
  await expect(first).toBeVisible();
});
