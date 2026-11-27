// The Compliance site in real browsers: compliance.html, the one file that
// `flightdeck compliance build --out` writes, opened from disk and inside a
// frame with Flare's planned policy. It needs target/debug/flightdeck,
// built after `pnpm run build`, so the binary embeds the page.
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { type Browser, chromium, expect, type Page, webkit } from "@playwright/test";
import { sampleJson } from "../fixtures/compliance/sample";
import { expectNoAccessibilityViolations, test } from "./checks";

const ui = fileURLToPath(new URL("../..", import.meta.url));
const repository = join(ui, "..");
const artboards = join(ui, "tests/fixtures/compliance/artboards");

/** The policy Flare's frame sends with an HTML share: inline script and style, data: images, nothing else. */
const FLARE_POLICY =
  "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:";

/** Writes the page the binary builds from the repository's fixture reports, and returns its path. */
function builtByFlightdeck(): string {
  const out = mkdtempSync(join(tmpdir(), "compliance-site-"));
  execFileSync(join(repository, "target/debug/flightdeck"), [
    "compliance",
    "build",
    "--config",
    join(repository, "tests/fixtures/compliance/compliance.toml"),
    "--reports",
    join(repository, "tests/fixtures/compliance/reports"),
    "--built-at",
    "2026-11-25T23:13:20Z",
    "--out",
    out,
  ]);
  return join(out, "compliance.html");
}

/**
 * Writes the built page with `json` inside it, as src/compliance/site.rs
 * writes a document, and returns its path. The sample draws the signed
 * artboards' repositories, which the binary's fixtures do not hold.
 */
function pageWith(json: unknown): string {
  const template = readFileSync(join(ui, "dist/compliance.html"), "utf8");
  const element = `<script type="application/json" id="compliance-document">${JSON.stringify(
    json,
  ).replaceAll("<", "\\u003c")}</script>`;
  const out = mkdtempSync(join(tmpdir(), "compliance-sample-"));
  const path = join(out, "compliance.html");
  writeFileSync(
    path,
    template.replace("<!--flightdeck:compliance-document-->", () => element),
  );
  return path;
}

const sample = pageWith(sampleJson);

/** Records every request `page` makes after the first one, which loads the file itself. */
function requestsAfterLoad(page: Page): string[] {
  const seen: string[] = [];
  page.on("request", (request) => seen.push(request.url()));
  return seen;
}

/** Opens the Compliance page, the Rules page and one repository's page by their links, checking each draws. */
async function visitEveryPage(page: Page) {
  await expect(page.getByRole("heading", { level: 1 })).toContainText(/fail|current/);
  await page.getByRole("link", { name: "Rules", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toContainText(/rules? reach/);
  await page.getByRole("link", { name: "Compliance", exact: true }).click();
  const matrix = page.getByRole("region", { name: /Rules and checks by repository/ });
  await matrix.getByRole("columnheader").nth(2).getByRole("link").click();
  await expect(page.getByRole("heading", { level: 1 })).toContainText(/owner\/app/);
}

for (const [name, engine] of [
  ["Chromium", chromium],
  ["WebKit", webkit],
] as const) {
  test(`in ${name}, the built file opens from disk and draws every page with no other request`, async () => {
    const browser: Browser = await engine.launch();
    try {
      const page = await browser.newPage();
      const file = pathToFileURL(builtByFlightdeck()).href;
      const requests = requestsAfterLoad(page);
      const errors: string[] = [];
      page.on("pageerror", (error) => errors.push(error.message));
      await page.goto(file);
      await visitEveryPage(page);
      expect(requests.map((url) => url.replace(/#.*$/, ""))).toEqual([file]);
      expect(errors).toEqual([]);
    } finally {
      await browser.close();
    }
  });
}

/**
 * Serves `html` as a Flare share inside a Flare host page at `page`, and
 * returns the URLs the page requested and the policy violations any frame
 * reported, each as "<directive> <blocked URI>". The listener is an init
 * script, which Playwright runs in every frame before the frame's own code.
 */
async function servedAsShare(page: Page, html: string) {
  const host = "https://flare.test/";
  const share = "https://share.flare.test/view";
  const requests: string[] = [];
  const violations: string[] = [];
  page.on("request", (request) => requests.push(request.url()));
  page.on("console", (message) => {
    const text = message.text();
    if (text.startsWith("csp-violation ")) violations.push(text.slice("csp-violation ".length));
  });
  await page.addInitScript(() => {
    document.addEventListener("securitypolicyviolation", (event) => {
      // biome-ignore lint/suspicious/noConsole: the frame's console carries each violation to the test
      console.info(`csp-violation ${event.effectiveDirective} ${event.blockedURI}`);
    });
  });
  await page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url === host) {
      await route.fulfill({
        contentType: "text/html",
        body: `<!doctype html><title>Flare</title><iframe title="share" sandbox="allow-scripts" src="${share}" style="width:1360px;height:900px"></iframe>`,
      });
    } else if (url === share) {
      await route.fulfill({
        contentType: "text/html",
        headers: { "Content-Security-Policy": FLARE_POLICY },
        body: html,
      });
    } else {
      await route.abort();
    }
  });
  await page.goto(host);
  return { host, share, requests, violations, frame: page.frameLocator('iframe[title="share"]') };
}

test("inside a frame with Flare's policy, the file draws every page with no violation and no request", async ({
  page,
}) => {
  const html = readFileSync(builtByFlightdeck(), "utf8");
  const { host, share, requests, violations, frame } = await servedAsShare(page, html);
  await expect(frame.getByRole("heading", { level: 1 })).toContainText("checks fail");
  await frame.getByRole("link", { name: "Rules", exact: true }).click();
  await expect(frame.getByRole("heading", { level: 1 })).toContainText("rules reach");
  await frame.getByRole("link", { name: "Compliance", exact: true }).click();
  const matrix = frame.getByRole("region", { name: /Rules and checks by repository/ });
  await matrix.getByRole("columnheader").nth(2).getByRole("link").click();
  await expect(frame.getByRole("heading", { level: 1 })).toContainText("owner/app");
  expect(violations).toEqual([]);
  expect(requests).toEqual([host, share]);
});

test("inside the same frame, a page that loads an image from the network reports the violation", async ({
  page,
  errors,
}) => {
  // The control for the test above: the listener sees what the policy blocks.
  const html = `<!doctype html><title>control</title><h1>control</h1><img alt="" src="https://images.flare.test/a.png">`;
  const { violations, frame } = await servedAsShare(page, html);
  await expect(frame.getByRole("heading", { level: 1 })).toHaveText("control");
  await expect.poll(() => violations).toEqual(["img-src https://images.flare.test/a.png"]);
  // The browser also logs the violation as an error, which this test expects.
  expect(errors.every((error) => error.includes("Content Security Policy"))).toBe(true);
  errors.length = 0;
});

test("a filtered address reloads the same view", async ({ page }) => {
  await page.goto(pathToFileURL(sample).href);
  await page.getByRole("button", { name: /^Failing/ }).click();
  await page.getByRole("button", { name: "Platform" }).click();
  await page.reload();
  await expect(page.getByRole("button", { name: /^Failing/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(page.getByRole("button", { name: "Platform" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  const heads = page
    .getByRole("region", { name: /Rules and checks by repository/ })
    .getByRole("columnheader");
  await expect(heads).toHaveCount(5);
});

for (const [label, hash] of [
  ["Compliance", "#/"],
  ["Rules", "#/rules"],
  ["repository", "#/repositories/data-pipeline"],
] as const) {
  test(`the ${label} page passes axe`, async ({ page }) => {
    await page.goto(`${pathToFileURL(sample).href}${hash}`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expectNoAccessibilityViolations(page);
  });
}

test("the report's run link draws with no underline until hovered, as artboard 55 draws it, and the breadcrumb keeps its underline", async ({
  page,
}) => {
  await page.goto(`${pathToFileURL(sample).href}#/repositories/data-pipeline`);
  const run = page.getByRole("link", { name: "main run #301" });
  await expect(run).toHaveCSS("text-decoration-line", "none");
  await run.hover();
  await expect(run).toHaveCSS("text-decoration-line", "underline");
  const crumb = page.locator(".cs-sub").getByRole("link", { name: "Compliance" });
  await expect(crumb).toHaveCSS("text-decoration-line", "underline");
});

/**
 * Returns the share of pixels that differ between two PNG screenshots, over
 * the area both cover: a pixel differs when a channel is more than 48 of 255
 * apart. The comparison runs in the page, which decodes the images.
 */
async function differingShare(page: Page, a: Buffer, b: Buffer): Promise<number> {
  return page.evaluate(
    async ([first, second]) => {
      const load = async (base64: string) => {
        const image = new Image();
        image.src = `data:image/png;base64,${base64}`;
        await image.decode();
        return image;
      };
      const [one, two] = await Promise.all([load(first), load(second)]);
      const width = Math.min(one.width, two.width);
      const height = Math.min(one.height, two.height);
      const pixels = (image: HTMLImageElement) => {
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d");
        if (context === null) throw new Error("no canvas");
        context.drawImage(image, 0, 0);
        return context.getImageData(0, 0, width, height).data;
      };
      const p = pixels(one);
      const q = pixels(two);
      let differ = 0;
      for (let at = 0; at < p.length; at += 4) {
        for (let channel = 0; channel < 3; channel += 1) {
          if (Math.abs((p[at + channel] ?? 0) - (q[at + channel] ?? 0)) > 48) {
            differ += 1;
            break;
          }
        }
      }
      return differ / (width * height);
    },
    [a.toString("base64"), b.toString("base64")] as const,
  );
}

/**
 * The most of a page's pixels that may differ from its artboard. The sample
 * holds fewer rules than the artboards show and words the build writes in
 * place of the artboards' sample sentences, so some rows and columns move.
 */
const MOST_DIFFERING = 0.1;

/** The parts of a page compared with the same parts of its artboard: the artboard's selector, then the page's. */
type Parts = readonly (readonly [string, string])[];

const TOP: Parts = [
  [".top", ".cs-top"],
  [".hero", ".cs-hero"],
];

for (const [label, artboard, hash, parts] of [
  [
    "Compliance",
    "compliance.html",
    "#/",
    [...TOP, [".tiles", ".cs-tiles"], [".main > .card", "section[aria-labelledby=matrix-heading]"]],
  ],
  [
    "repository",
    "repository.html",
    "#/repositories/data-pipeline",
    [
      ...TOP,
      [".grid .colm .card", "section[aria-labelledby=measured-heading]"],
      [".grid > .colm:last-child > .card:last-child", "section[aria-labelledby=legend-heading]"],
    ],
  ],
  [
    "Rules",
    "rules.html",
    "#/rules",
    [
      [".main > .tiles", ".cs-tiles"],
      [".main > .card", "section[aria-labelledby=reports-heading]"],
      [".main > .segs", ".cs-segs"],
      [".main > .cols3", ".cs-cols3"],
    ],
  ],
] as const satisfies readonly (readonly [string, string, string, Parts])[]) {
  test(`the ${label} page matches its signed artboard`, async ({ page }) => {
    // The artboards are drawn in the light appearance, so both projects compare in it.
    await page.emulateMedia({ colorScheme: "light" });
    // Artboard 52 is the live page's main column, 1,144 px wide beside its rail.
    await page.setViewportSize({ width: label === "Rules" ? 1144 : 1360, height: 900 });
    // The artboards are kept as text, so the linter leaves their markup as signed.
    await page.setContent(readFileSync(join(artboards, `${artboard}.txt`), "utf8"));
    const drawn: Buffer[] = [];
    for (const [selector] of parts) drawn.push(await page.locator(selector).first().screenshot());
    await page.goto(`${pathToFileURL(sample).href}${hash}`);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    const shot = async () => {
      const shots = [];
      for (const [, selector] of parts)
        shots.push(await page.locator(selector).first().screenshot());
      return shots;
    };
    const mean = async (shots: readonly Buffer[]) => {
      let sum = 0;
      for (const [at, built] of shots.entries()) {
        const artboard = drawn[at];
        if (artboard === undefined) throw new Error(`no artboard part ${at}`);
        sum += await differingShare(page, artboard, built);
      }
      return sum / shots.length;
    };
    const share = await mean(await shot());
    // The control: the same parts with none of the site's styles, which a
    // comparison that cannot tell the livery apart would also pass.
    await page.evaluate(() => {
      for (const style of document.querySelectorAll("style")) style.remove();
    });
    const control = await mean(await shot());
    test.info().annotations.push({
      type: "differing share",
      description: `${share.toFixed(3)} against ${control.toFixed(3)} unstyled`,
    });
    expect(share).toBeLessThan(MOST_DIFFERING);
    expect(share).toBeLessThan(control / 2);
  });
}
