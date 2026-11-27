import type { ComplianceSource } from "@crumbkit/flightdeck-headless/source";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  REFUSAL_CODE,
  refusedSampleJson,
  sampleJson,
} from "../../tests/fixtures/compliance/sample.ts";
import { DOCUMENT_ID, inlineSource } from "./inlineSource.ts";
import { Site } from "./Site.tsx";

/** Returns `value`, failing the test when it is missing. */
function present<T>(value: T | undefined): T {
  if (value === undefined) throw new Error("expected a value");
  return value;
}

/** A source that reads `json` as the page's document element would hold it. */
function sourceOf(json: unknown): ComplianceSource {
  return { read: async () => structuredClone(json) };
}

/** Draws the site at `hash` over `json`, the sample by default. */
async function drawAt(hash: string, json: unknown = sampleJson) {
  window.location.hash = hash;
  const view = render(<Site source={sourceOf(json)} />);
  await screen.findByRole("heading", { level: 1 });
  return view;
}

/** Returns the matrix's table. */
function matrix() {
  const card = screen.getByRole("region", { name: /Rules and checks by repository/ });
  return within(card).getByRole("table");
}

/** Returns the names in the first cell of each of the matrix's rule rows. */
function matrixRowNames() {
  return within(matrix())
    .getAllByRole("row")
    .slice(1)
    .filter((row) => !row.classList.contains("cs-grp"))
    .map((row) => within(row).queryAllByRole("cell")[0]?.textContent ?? "")
    .filter((name) => name !== "" && !name.startsWith("Show") && !name.startsWith("Hide"));
}

beforeEach(() => {
  window.location.hash = "";
});

afterEach(() => {
  window.location.hash = "";
});

describe("the Compliance site", () => {
  it("draws one line with the code when the page holds no document", async () => {
    window.location.hash = "";
    const page = document.implementation.createHTMLDocument("empty");
    const { container } = render(<Site source={inlineSource(page)} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toMatch(/^flightdeck::compliance::no_document: /);
    expect(container.children).toHaveLength(1);
  });

  it("draws one line with the field that failed when the document fails the schema", async () => {
    const broken: { data: { rows: { section: string }[] } } = JSON.parse(
      JSON.stringify(sampleJson),
    );
    const first = broken.data.rows[0];
    if (first !== undefined) first.section = "sideways";
    const { container } = render(<Site source={sourceOf(broken)} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toMatch(
      /^flightdeck::compliance::invalid_document: data\.rows\.0\.section/,
    );
    expect(container.querySelectorAll("table")).toHaveLength(0);
  });

  it("reads the document from the element compliance build writes", async () => {
    const page = document.implementation.createHTMLDocument("site");
    const element = page.createElement("script");
    element.type = "application/json";
    element.id = DOCUMENT_ID;
    element.textContent = JSON.stringify(sampleJson);
    page.body.append(element);
    render(<Site source={inlineSource(page)} />);
    expect(
      await screen.findByRole("heading", { level: 1, name: /checks fail in/ }),
    ).toBeInTheDocument();
  });

  it("names the snapshot's time in the top bar", async () => {
    await drawAt("#/");
    expect(screen.getByRole("banner")).toHaveTextContent("Snapshot of 25 Nov 2026, 06:00 UTC");
  });
});

describe("the Compliance page", () => {
  it("draws each cell state as a word in its colour", async () => {
    await drawAt("#/");
    const table = matrix();
    for (const [word, colour] of [
      ["failing", "cs-failing"],
      ["stale", "cs-stale"],
      ["current", "cs-current"],
    ] as const) {
      const chips = within(table).getAllByText(word, { selector: "span.cs-chip" });
      expect(chips.length).toBeGreaterThan(0);
      for (const chip of chips) expect(chip).toHaveClass(colour);
    }
    expect(within(table).getAllByText("–").length).toBeGreaterThan(0);
  });

  it("says no report in each cell of a column with no report, and draws the other columns", async () => {
    await drawAt("#/");
    const head = within(matrix()).getByRole("link", { name: /^mobile-app, no report/ });
    expect(head).toBeInTheDocument();
    const rows = within(matrix()).getAllByRole("row");
    const agentRow = rows.find((row) => row.textContent?.startsWith("Agent context"));
    const cells = within(present(agentRow)).getAllByRole("cell");
    expect(cells.map((cell) => cell.textContent)).toEqual([
      "Agent context stays under 1,000 words",
      "budget · words",
      "current",
      "failing1,212",
      "current",
      "stale9 d",
      "no report",
      "current",
      "current",
      "current",
    ]);
  });

  it("keeps only the rows with a failing cell once the Failing tile is pressed, in the address", async () => {
    const user = userEvent.setup();
    await drawAt("#/");
    const all = matrixRowNames().length;
    await user.click(screen.getByRole("button", { name: /^Failing/ }));
    expect(window.location.hash).toBe("#/?tile=failing");
    expect(screen.getByRole("button", { name: /^Failing/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    const failing = matrixRowNames();
    expect(failing.length).toBeLessThan(all);
    expect(failing).toContain("A Python file stays under 40 KB");
    expect(failing).not.toContain("Terraform is at least 1.9");
  });

  it("draws the same filtered view when the page opens on a filtered address", async () => {
    await drawAt("#/?show=not_current&tile=failing&group=Platform");
    expect(screen.getByRole("button", { name: "Not current" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Platform" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    const heads = within(matrix())
      .getAllByRole("columnheader")
      .slice(2)
      .map((head) => within(head).getByRole("link").textContent);
    expect(heads).toEqual(["infra-terraform", "data-pipeline", "docs-site"]);
    expect(matrixRowNames()).toEqual([
      "A Python file stays under 40 KB",
      "Node is at least 22.12",
      "Python standard audit",
    ]);
  });

  it("keeps the tile when the Show chips change", async () => {
    const user = userEvent.setup();
    await drawAt("#/?tile=stale");
    await user.click(screen.getByRole("button", { name: "Not current" }));
    expect(window.location.hash).toBe("#/?show=not_current&tile=stale");
  });

  it("opens the written rules when their button is pressed", async () => {
    const user = userEvent.setup();
    await drawAt("#/");
    const button = screen.getByRole("button", { name: /^Show 7 written rules/ });
    expect(screen.queryByText("Test names are sentences")).not.toBeVisible();
    await user.click(button);
    expect(screen.getByText("Test names are sentences")).toBeVisible();
  });

  it("draws a requirement that holds a closing script tag as text", async () => {
    const json: { data: { rows: { requirement: string | null }[] } } = JSON.parse(
      JSON.stringify(sampleJson),
    );
    const first = json.data.rows[0];
    if (first !== undefined) first.requirement = "</script><img src=x onerror=alert(1)>";
    const { container } = await drawAt("#/", json);
    expect(screen.getAllByText("</script><img src=x onerror=alert(1)>").length).toBeGreaterThan(0);
    expect(container.querySelector("img")).toBeNull();
  });

  it("writes TypeScript with its capitals in the matrix and the stale list", async () => {
    await drawAt("#/");
    expect(within(matrix()).getByText("TypeScript standard audit")).toBeInTheDocument();
    const stale = screen.getByRole("region", { name: /^Stale checks/ });
    expect(within(stale).getByText(/TypeScript standard audit/)).toBeInTheDocument();
    expect(screen.queryByText(/Typescript/)).toBeNull();
  });

  it("gives a zero rule that fails in two repositories one Failing item naming both", async () => {
    await drawAt("#/");
    const failing = screen.getByRole("region", { name: /^Failing · / });
    const items = [...failing.querySelectorAll(".cs-item")].map((item) => item.textContent);
    expect(items.filter((text) => text?.includes("v1 webhook signer"))).toEqual([
      "payments-api, billing-worker · No service still calls the v1 webhook signerThe newest run reports remaining 3 and unknown 0.Run #1184",
    ]);
  });

  it("says what a pin behind the rule pins, with its file in the mono face", async () => {
    await drawAt("#/");
    const failing = screen.getByRole("region", { name: /^Failing · / });
    const note = within(failing).getByText(/line 1 pins 20\.18\.0/);
    expect(note.textContent).toBe(".nvmrc line 1 pins 20.18.0.");
    expect(within(note).getByText(".nvmrc")).toHaveClass("cs-mono");
  });

  it("sets the paths, commits and measures in the lists' notes in the mono face", async () => {
    await drawAt("#/");
    const failing = screen.getByRole("region", { name: /^Failing · / });
    for (const text of ["pipeline/transform/joins.py", "AGENTS.md", "remaining 3", "7f20c4d"]) {
      expect(within(failing).getByText(text)).toHaveClass("cs-mono");
    }
    const stale = screen.getByRole("region", { name: /^Stale checks/ });
    expect(within(stale).getByText("/suites/docs-site-typescript-standard")).toHaveClass("cs-mono");
  });

  it("lists a repository with no report before one whose report is stale", async () => {
    await drawAt("#/");
    const missing = screen.getByRole("region", { name: /Repositories without a current report/ });
    const names = within(missing)
      .getAllByRole("link")
      .filter((link) => link.classList.contains("cs-mono"))
      .map((link) => link.textContent);
    expect(names).toEqual(["mobile-app", "identity-service"]);
  });

  it("links each cell to its repository's page with its rule selected", async () => {
    const user = userEvent.setup();
    await drawAt("#/");
    await user.click(within(matrix()).getByRole("link", { name: /^data-pipeline, A Python file/ }));
    expect(window.location.hash).toBe(
      "#/repositories/data-pipeline?rule=%2Frules%2Fpython%2Ffile-size",
    );
    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "data-pipeline fails 1 rule and 1 audit",
      }),
    ).toBeInTheDocument();
  });
});

describe("a repository's page", () => {
  it("draws each rule with its measure, the commit checked and the report's time", async () => {
    await drawAt("#/repositories/data-pipeline?rule=%2Frules%2Fpython%2Ffile-size");
    const report = screen.getByText((_, element) =>
      Boolean(element?.matches("span.cs-cap") && element.textContent?.startsWith("Report from")),
    );
    expect(report.textContent).toBe(
      "Report from main run #301 at 7f20c4d, made at 00:00 UTC on 25 November 2026, 6 h before the snapshot",
    );
    const measured = screen.getByRole("region", { name: /Rules measured at 7f20c4d · 3/ });
    expect(
      within(measured)
        .getAllByRole("cell")
        .map((cell) => cell.textContent),
    ).toContain("1 file over: pipeline/transform/joins.py 46,880");
    const detail = screen.getByRole("region", { name: /^Failing · A Python file/ });
    expect(within(detail).getByText("/rules/python/file-size")).toBeInTheDocument();
    expect(
      within(detail).getByText("board rules report --path /repos/data-pipeline"),
    ).toBeInTheDocument();
  });

  it("lists the failing rule first and sets the paths it measured in the mono face", async () => {
    await drawAt("#/repositories/data-pipeline");
    const measured = screen.getByRole("region", { name: /Rules measured at 7f20c4d · 3/ });
    const names = within(measured)
      .getAllByRole("row")
      .slice(1)
      .map((row) => within(row).getAllByRole("cell")[0]?.textContent);
    expect(names).toEqual([
      "A Python file stays under 40 KB",
      "Agent context stays under 1,000 words",
      "Python is at least 3.12",
    ]);
    expect(within(measured).getByText("pipeline/transform/joins.py")).toHaveClass("cs-mono");
  });

  it("heads the failing panel's table with the budget's unit and names the files it reads from the root", async () => {
    await drawAt("#/repositories/data-pipeline?rule=%2Frules%2Fpython%2Ffile-size");
    const detail = screen.getByRole("region", { name: /^Failing · A Python file/ });
    expect(within(detail).getByRole("columnheader", { name: "Bytes" })).toBeInTheDocument();
    const value = (term: string) => {
      const given = within(detail).getByText(term).nextElementSibling;
      if (!(given instanceof HTMLElement)) throw new Error(`no value for ${term}`);
      return given;
    };
    expect(value("Reads").textContent?.startsWith("/")).toBe(false);
    expect(within(value("Reads")).getByText(/^pipeline\//)).toHaveClass("cs-mono");
    expect(within(value("Measured at")).getByText("7f20c4d")).toHaveClass("cs-mono");
  });

  it("names the audit that judges the written rules and the age of its run", async () => {
    await drawAt("#/repositories/data-pipeline");
    const written = screen.getByRole("region", { name: /^Written rules · / });
    expect(within(written).getByText("Judged by the Python standard audit, 1 d ago")).toBeVisible();
  });

  it("moves to another rule's detail when its name is pressed", async () => {
    const user = userEvent.setup();
    await drawAt("#/repositories/data-pipeline");
    await user.click(screen.getByRole("button", { name: "Python is at least 3.12" }));
    expect(window.location.hash).toBe(
      "#/repositories/data-pipeline?rule=%2Frules%2Ftoolchains%2Fpython",
    );
    expect(screen.getByRole("region", { name: "Current · Python is at least 3.12" })).toHaveFocus();
  });

  it("says a repository with no report sent none", async () => {
    await drawAt("#/repositories/mobile-app");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "mobile-app sent no report",
    );
  });

  it("says a repository the snapshot lacks is not in it", async () => {
    await drawAt("#/repositories/nowhere");
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "This snapshot has no repository nowhere",
    );
  });
});

describe("the Rules page", () => {
  it("filters the rules by kind, by when they are shown and by state, in the address", async () => {
    const user = userEvent.setup();
    await drawAt("#/rules");
    const table = () => screen.getByRole("region", { name: /^Rules · / });
    expect(within(table()).getByRole("heading")).toHaveTextContent("Rules · 14");
    await user.click(screen.getByRole("button", { name: "obligation" }));
    expect(window.location.hash).toBe("#/rules?kind=obligation");
    expect(within(table()).getByRole("heading")).toHaveTextContent("Rules · 1");
    await user.click(screen.getByRole("button", { name: "All", pressed: false }));
    await act(async () => {});
    await user.click(screen.getByRole("button", { name: "start" }));
    expect(within(table()).getByRole("heading")).toHaveTextContent("Rules · 1");
    await user.click(present(screen.getAllByRole("button", { name: "All" })[2]));
    await user.click(present(screen.getAllByRole("button", { name: "Not current" })[0]));
    expect(window.location.hash).toContain("state=not_current");
    const names = within(table())
      .getAllByRole("row")
      .slice(1)
      .map((row) => row.querySelector(".cs-sub2")?.textContent);
    expect(names).toContain("/rules/python/file-size");
    expect(names).not.toContain("/rules/toolchains/terraform");
  });

  it("lists the five checkers of the signed Rules artboard in its Checker chips and its Checkers card", async () => {
    await drawAt("#/rules");
    const chips = screen.getByRole("group", { name: "Checker" });
    expect(
      within(chips)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["All", "budget", "version", "pattern", "judged", "written"]);
    const card = screen.getByRole("region", { name: /^Checkers/ });
    expect(within(card).queryByText("zero")).toBeNull();
  });

  it("names what a pattern rule quotes beside its checker", async () => {
    await drawAt("#/rules");
    const row = screen
      .getAllByRole("row")
      .find((each) => each.textContent?.includes("/rules/privacy/customer-names"));
    expect(within(present(row)).getByText("pattern · match-only")).toBeInTheDocument();
  });

  it("lists each repository's report with its run, commit and age", async () => {
    await drawAt("#/rules");
    const reports = screen.getByRole("region", { name: /Reports by repository · 8 configured/ });
    const row = within(reports)
      .getAllByRole("row")
      .find((each) => each.textContent?.startsWith("data-pipeline"));
    expect(row?.textContent).toBe("data-pipelinemain run #3017f20c4d6 h9current");
    expect(within(present(row)).getByRole("link")).toHaveAttribute(
      "href",
      "https://github.com/platform/data-pipeline/actions/runs/7000301",
    );
  });
});

describe("a document with a refused report and an unknown checker", () => {
  it("names a checker this build does not know as an unknown checker with its name", async () => {
    await drawAt("#/", refusedSampleJson());
    const row = within(matrix())
      .getAllByRole("row")
      .find((each) => each.textContent?.toLowerCase().includes("changelog"));
    expect(within(present(row)).getAllByRole("cell")[1]).toHaveTextContent("unknown checker regex");
  });

  it("shows a refused report's code in its column and still draws the other columns", async () => {
    await drawAt("#/", refusedSampleJson());
    const head = within(matrix()).getByRole("link", { name: /^legacy-cron, invalid report/ });
    expect(present(head.closest("th") ?? undefined)).toHaveTextContent("invalid_report");
    const missing = screen.getByRole("region", { name: /Repositories without a current report/ });
    expect(missing).toHaveTextContent("flightdeck::compliance::invalid_report");
    const row = within(matrix())
      .getAllByRole("row")
      .find((each) => each.textContent?.includes("Python file stays"));
    const cells = within(present(row)).getAllByRole("cell");
    expect(cells.map((cell) => cell.textContent)).toContain(`not readable${REFUSAL_CODE}`);
    const links = within(present(row))
      .getAllByRole("link")
      .map((link) => link.getAttribute("href"));
    expect(links).toContain("#/repositories/payments-api?rule=%2Frules%2Fpython%2Ffile-size");
    expect(links).toContain("#/repositories/data-pipeline?rule=%2Frules%2Fpython%2Ffile-size");
  });
});
