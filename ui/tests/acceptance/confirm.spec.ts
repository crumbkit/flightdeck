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

/**
 * Chooses books remove in the rail, enters the ids and presses Preview.
 * Returns the card by its id, because its name changes with its status.
 */
async function previewRemove(page: Page, ids: string) {
  await page
    .getByRole("navigation", { name: "Commands" })
    .getByRole("button", { name: /books remove/ })
    .click();
  const composer = page.getByRole("form", { name: "Composer" });
  await composer.getByLabel(/The ids of the books to remove/).pressSequentially(`${ids} `);
  await composer.getByRole("button", { name: /Preview/ }).click();
  await expect(page.getByRole("region", { name: /^Run 1: .*awaiting confirmation/ })).toBeVisible();
  return page.locator("#run-1");
}

/** Counts the requests the page sends to the confirmation route. */
function countConfirmations(page: Page): { count: number } {
  const seen = { count: 0 };
  page.on("request", (request) => {
    if (new URL(request.url()).pathname.startsWith("/api/confirmations/")) {
      seen.count += 1;
    }
  });
  return seen;
}

test("books remove 2 previews Frankenstein and changes nothing", async ({ page, served }) => {
  await signIn(page, served);

  const card = await previewRemove(page, "2");

  await expect(card.getByText("This was a dry run, so nothing changed.")).toBeVisible();
  await expect(card.getByText("Frankenstein", { exact: true })).toBeVisible();
  await expect(card.getByRole("group", { name: "Remove 1 book?" })).toBeVisible();
  await expect(card.getByText("Destructive. Idempotent.")).toBeVisible();
  // The pointer still rests on Preview, whose hover colour is not the
  // subject of this check.
  await page.mouse.move(0, 0);
  await expectNoAccessibilityViolations(page);
  await page.getByRole("button", { name: "Print the first page of books" }).click();
  const list = page.getByRole("region", { name: /^Run 2: shelf books list/ });
  await expect(list.getByText("Showing 4 of 6 books")).toBeVisible();
});

test("Enter on the preview card cancels", async ({ page, served }) => {
  const confirmations = countConfirmations(page);
  await signIn(page, served);
  const card = await previewRemove(page, "2");
  await expect(card.getByRole("button", { name: "Cancel" })).toBeFocused();

  await page.keyboard.press("Enter");

  await expect(page.getByRole("region", { name: /^Run 1: .*not confirmed/ })).toBeVisible();
  await expect(card.getByText("Not confirmed, so nothing changed.")).toBeVisible();
  expect(confirmations.count).toBe(0);
});

test("⌘↩ never confirms", async ({ page, served }) => {
  const confirmations = countConfirmations(page);
  await signIn(page, served);
  const card = await previewRemove(page, "2");

  await card.getByRole("button", { name: "Remove 1 book" }).focus();
  await page.keyboard.press("ControlOrMeta+Enter");

  await expect(card.getByRole("group", { name: "Remove 1 book?" })).toBeVisible();
  expect(confirmations.count).toBe(0);
});

test("editing the ids after a preview disables the confirm button", async ({ page, served }) => {
  await signIn(page, served);
  const card = await previewRemove(page, "2");

  await page
    .getByRole("form", { name: "Composer" })
    .getByLabel(/The ids of the books to remove/)
    .pressSequentially("3 ");

  await expect(card.getByRole("button", { name: "Remove 1 book" })).toBeDisabled();
  await expect(card.getByText(/Stale: the composer changed after this preview/)).toBeVisible();
});

test("confirm runs books remove 2 --yes once and the card turns into a result card", async ({
  page,
  served,
}) => {
  const confirmations = countConfirmations(page);
  await signIn(page, served);
  const card = await previewRemove(page, "2");
  await expect(card.getByText("shelf books remove 2 --yes", { exact: true })).toBeVisible();

  await card.getByRole("button", { name: "Remove 1 book" }).click();

  const result = page.getByRole("region", { name: /^Run 1: shelf books remove 2, exit 0/ });
  await expect(result).toBeVisible();
  await expect(result.getByText("Frankenstein")).toBeVisible();
  await expect(result.getByRole("group")).toHaveCount(0);
  await expect(result.getByText("This was a dry run, so nothing changed.")).toHaveCount(0);
  expect(confirmations.count).toBe(1);
  await page.getByRole("button", { name: "Print every book" }).click();
  const list = page.getByRole("region", { name: /^Run 2: shelf books list --all, exit 0/ });
  await expect(list.getByRole("cell", { name: "Moby-Dick" })).toBeVisible();
  await expect(list.getByText("Frankenstein")).toHaveCount(0);
});

test.describe("with --read-only", () => {
  test.use({ readOnly: true });

  test("books remove is in the rail, dimmed, with no buttons", async ({ page, served }) => {
    await signIn(page, served);
    await expect(page.getByRole("banner").getByText("read-only", { exact: true })).toBeVisible();
    const remove = page
      .getByRole("navigation", { name: "Commands" })
      .getByRole("button", { name: /books remove/ });
    await expect(remove).toContainText("turned off by --read-only");

    await remove.click();

    const composer = page.getByRole("form", { name: "Composer" });
    await expect(
      composer.getByText(
        "Turned off by --read-only. Start flightdeck without it to run this command.",
      ),
    ).toBeVisible();
    await expect(composer.getByRole("button", { name: /Preview|Run/ })).toHaveCount(0);
    await expectNoAccessibilityViolations(page);
  });
});
