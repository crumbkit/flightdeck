import { readFile, writeFile } from "node:fs/promises";
import { expect, type Locator, type Page } from "@playwright/test";
import { expectNoAccessibilityViolations } from "./checks";
import { writeList } from "./reading";
import { type Served, test } from "./real";

// The page's address holds the launch code until the page removes it, so
// no trace is kept of these tests. The column picker's test reads the
// copied line from the clipboard.
test.use({ trace: "off", permissions: ["clipboard-read", "clipboard-write"] });

/** Opens the opener page and waits for the signed-in page of `tool`. */
async function signIn(page: Page, served: Served, tool = "shelf"): Promise<void> {
  await page.goto(served.opener);
  await expect(page.getByRole("region", { name: `Welcome to ${tool}` })).toBeVisible();
}

/**
 * Presses the composer's Run, and moves the pointer off it, so an
 * accessibility check reads the button's own colours rather than its hover
 * colour.
 */
async function pressRun(page: Page, composer: Locator): Promise<void> {
  await composer.getByRole("button", { name: /^Run/ }).click();
  await page.mouse.move(0, 0);
}

/** Opens a command from the rail in the composer, and returns the composer. */
async function compose(page: Page, path: string) {
  await page
    .getByRole("navigation", { name: "Commands" })
    .getByRole("button", { name: new RegExp(`^${path}`) })
    .click();
  const composer = page.getByRole("form", { name: "Composer" });
  await expect(composer.getByRole("heading")).toHaveText(path);
  return composer;
}

/** The fixture that the unit tests read. */
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

test("the rail shows shelf's three goal headings in help order", async ({ page, served }) => {
  await signIn(page, served);
  const rail = page.getByRole("navigation", { name: "Commands" });

  await expect(rail.getByRole("heading")).toHaveText([
    "Browse the catalogue",
    "Explore the commands",
    "Review how shelf is used",
  ]);
  await expect(rail.getByRole("button")).toHaveCount(9);
  await expect(rail.getByRole("button", { name: "books remove destructive" })).toBeVisible();
  await expectNoAccessibilityViolations(page);
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

test("a pasted line fills the form, Enter runs it, and the card shows the long columns it asked for", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  const line = page.getByRole("textbox", { name: /Command line/ });

  await line.fill("$ shelf books show 2 5 --long");
  await line.press("Enter");

  await expect(line).toHaveValue("books show 2 5 --long");
  const card = page.getByRole("region", { name: /^Run 1: shelf books show 2 5, exit 0/ });
  await expect(card.getByRole("checkbox", { name: "Long columns" })).toBeChecked();
  await expect(card.getByRole("columnheader").filter({ hasText: "reading" })).toHaveCount(1);
});

test("a refused word in the line is underlined with the tool's words and fixed with one click", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  const line = page.getByRole("textbox", { name: /Command line/ });

  await line.fill("books list --limti 2");
  await line.blur();

  await expect(page.getByText("unexpected argument '--limti' found")).toBeVisible();
  await expectNoAccessibilityViolations(page);
  await page.getByRole("button", { name: "Change to --limit" }).click();
  await expect(line).toHaveValue("books list --limit 2");
  await line.press("End");
  await line.press("Enter");
  // The card shows the server's line, which joins each option to its value.
  await expect(
    page.getByRole("region", { name: /^Run 1: shelf books list --limit=2, exit 0/ }),
  ).toBeVisible();
});

test("books show 2 7 9 draws Frankenstein, then a failure card for 7 and one for 9", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  const composer = await compose(page, "books show");
  await composer.getByLabel(/The ids of the books to show/).pressSequentially("2 7 9 ");
  await pressRun(page, composer);

  const card = page.getByRole("region", { name: /^Run 1: shelf books show 2 7 9, exit 1/ });
  await expect(card.getByText("Frankenstein")).toBeVisible();
  const failures = card.getByRole("alert");
  await expect(failures).toHaveCount(2);
  await expect(failures.nth(0)).toContainText("7 (index 1)the catalogue has no book 7");
  await expect(failures.nth(1)).toContainText("9 (index 2)the catalogue has no book 9");
  await expect(failures.nth(0)).toContainText("shelf::books::not_found");
  await expectNoAccessibilityViolations(page);
});

test("a failure's recovery chip opens books list in the composer and runs nothing", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  const composer = await compose(page, "books show");
  await composer.getByLabel(/The ids of the books to show/).pressSequentially("7 ");
  await composer.getByRole("button", { name: /^Run/ }).click();
  const card = page.getByRole("region", { name: /^Run 1: shelf books show 7, exit 1/ });

  await card.getByRole("button", { name: "books list" }).click();

  await expect(composer.getByRole("heading")).toHaveText("books list");
  await expect(page.getByRole("region", { name: /^Run 2/ })).toHaveCount(0);
});

test("books list --limit 101 marks the limit field with crumbkit's message", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  const composer = await compose(page, "books list");
  await composer.getByText("More options").click();
  await composer.getByLabel(/--limit/).fill("101");
  await pressRun(page, composer);

  const card = page.getByRole("region", { name: /^Run 1: shelf books list --limit=101, exit 2/ });
  await expect(card.getByText("--limit=101", { exact: true })).toHaveClass(/decoration-wavy/);
  const message = "--limit takes a whole number from 1 to 100";
  await expect(composer.getByLabel(/--limit/)).toHaveAttribute("aria-invalid", "true");
  await expect(composer.getByText(message, { exact: false })).toBeVisible();
  await expectNoAccessibilityViolations(page);
});

test("books list shows the next chip books show <ID>…, which opens books show", async ({
  page,
  served,
}) => {
  await signIn(page, served);

  await page.getByRole("button", { name: "Print the first page of books" }).click();
  const card = page.getByRole("region", { name: /^Run 1: shelf books list, exit 0/ });
  await card.getByRole("button", { name: "books show <ID>…" }).click();

  const composer = page.getByRole("form", { name: "Composer" });
  await expect(composer.getByRole("heading")).toHaveText("books show");
  await expect(composer.getByLabel(/The ids of the books to show/)).toBeFocused();
  await expect(page.getByRole("region", { name: /^Run 2/ })).toHaveCount(0);
});

test.describe("slow reading", () => {
  test.use({ tool: "slow reading" });

  /** Runs `reading entries stream --list books`, and returns its card. */
  async function streamBooks(page: Page, served: Served) {
    await writeList(served.data, "books", ["# Books", "Emma", "Persuasion", "Ivanhoe"]);
    await signIn(page, served, "reading");
    const composer = await compose(page, "entries stream");
    await composer.getByLabel(/The name of the reading list/).fill("books");
    await pressRun(page, composer);
    return page.getByRole("region", { name: /^Run 1: reading entries stream --list=books/ });
  }

  test("reading's stream appends rows as they arrive and announces its final status once", async ({
    page,
    served,
  }) => {
    const card = await streamBooks(page, served);

    await served.release();
    await served.release();
    await expect(card.getByRole("row")).toHaveCount(3);
    await expect(card.getByText("running · 2 rows", { exact: true })).toBeVisible();
    await served.release();

    await expect(card.getByRole("row")).toHaveCount(4);
    const live = card.getByRole("status");
    await expect(live).toHaveCount(1);
    await expect(live).toHaveText("exit 0 · 3 rows");
    await expectNoAccessibilityViolations(page);
  });

  test("Esc never cancels a run, and ⌘. cancels it and keeps its rows", async ({
    page,
    served,
  }) => {
    const card = await streamBooks(page, served);
    await served.release();
    await expect(card.getByRole("row")).toHaveCount(2);

    await page.keyboard.press("Escape");
    await served.release();
    await expect(card.getByRole("row")).toHaveCount(3);
    await page.keyboard.press("ControlOrMeta+.");

    await expect(
      card.getByText("cancelled · exit 130 · 2 rows kept", { exact: true }).first(),
    ).toBeVisible();
    await expect(card.getByRole("row")).toHaveCount(3);
  });
});

test.describe("pretty shelf", () => {
  test.use({ tool: "pretty shelf" });

  test("a shelf that indents its JSON draws books list as a table and books show as a record", async ({
    page,
    served,
  }) => {
    await signIn(page, served);

    await page.getByRole("button", { name: "Print the first page of books" }).click();
    const list = page.getByRole("region", { name: /^Run 1: shelf books list, exit 0/ });
    await expect(list.getByText("Showing 4 of 6 books")).toBeVisible();
    await expect(list.getByRole("row")).toHaveCount(5);
    const composer = await compose(page, "books show");
    await composer.getByLabel(/The ids of the books to show/).pressSequentially("2 ");
    await pressRun(page, composer);

    const show = page.getByRole("region", { name: /^Run 2: shelf books show 2, exit 0/ });
    await expect(show.getByText("Frankenstein")).toBeVisible();
    await expectNoAccessibilityViolations(page);
  });

  test("a shelf that indents its JSON fails books list --limit 101 with crumbkit's code and message", async ({
    page,
    served,
  }) => {
    await signIn(page, served);
    const composer = await compose(page, "books list");
    await composer.getByText("More options").click();
    await composer.getByLabel(/--limit/).fill("101");
    await pressRun(page, composer);

    const card = page.getByRole("region", { name: /^Run 1: shelf books list --limit=101, exit 2/ });
    await expect(card.getByText("shelf::cli::usage")).toBeVisible();
    await expect(composer.getByLabel(/--limit/)).toHaveAttribute("aria-invalid", "true");
  });
});

test.describe("reading", () => {
  test.use({ tool: "reading" });

  test("a result without next steps shows no next chip", async ({ page, served }) => {
    await writeList(served.data, "books", ["# Books", "Emma"]);
    await signIn(page, served, "reading");

    const composer = await compose(page, "lists list");
    await composer.getByRole("button", { name: /^Run/ }).click();

    const card = page.getByRole("region", { name: /^Run 1: reading lists list, exit 0/ });
    await expect(card.getByRole("cell", { name: "books", exact: true })).toBeVisible();
    await expect(card.getByText("Next:")).toHaveCount(0);
  });
});

/** Waits two frames, so effects that run after the page draws have run. */
async function settle(page: Page): Promise<void> {
  await page.evaluate(
    () => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))),
  );
}

test.describe("on a phone", () => {
  test.use({ viewport: { width: 375, height: 812 } });

  test("the page opens at the top of the welcome, not at the end of its command list", async ({
    page,
    served,
  }) => {
    await signIn(page, served);
    await settle(page);

    const welcome = page.getByRole("region", { name: "Welcome to shelf" });
    await expect(welcome.getByRole("heading", { level: 1 })).toBeInViewport();
  });
});

test("with a form open the composer takes at most 40% of the window, and the new card shows above it", async ({
  page,
  served,
}) => {
  await signIn(page, served);
  const composer = await compose(page, "books show");
  await composer.getByLabel(/The ids of the books to show/).pressSequentially("2 7 ");
  await pressRun(page, composer);

  const card = page.getByRole("region", { name: /^Run 1: shelf books show 2 7, exit 1/ });
  await expect(card).toBeVisible();
  await settle(page);
  const window = page.viewportSize();
  const dock = await composer.locator("..").boundingBox();
  if (window === null || dock === null) {
    throw new Error("the page has no viewport or the composer has no box");
  }
  expect(dock.height).toBeLessThanOrEqual(window.height * 0.4 + 1);
  await expect(composer.getByRole("button", { name: /^Run/ })).toBeInViewport();
  await expect(card.getByText("Run 1", { exact: true })).toBeInViewport();
  await expect(card.getByText("Frankenstein")).toBeInViewport();
});
