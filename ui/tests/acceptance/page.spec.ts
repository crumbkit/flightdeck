import { expect } from "@playwright/test";
import { shelfReply } from "../fixtures/shelf";
import { expectNoAccessibilityViolations, test } from "./checks";
import { openShelf } from "./stub";

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
