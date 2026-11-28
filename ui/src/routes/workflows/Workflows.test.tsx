import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeQueryClient } from "@/queryClient";
import {
  developmentFlow,
  developmentFlowOnOlderFlow,
  tangledFlow,
  workflowsList,
} from "../../../tests/fixtures/workflows";
import Workflows from "./Workflows";

/** Answers the workflows routes with the fake board's documents, or `flow` for the development flow. */
function stubServer(flow: object = developmentFlow) {
  vi.stubGlobal("fetch", async (path: string) => {
    if (path === "/api/workflows") {
      return Response.json(workflowsList);
    }
    if (path === "/api/workflows/tangled") {
      return Response.json(tangledFlow);
    }
    return Response.json(flow);
  });
}

/** Records what the page copies to the clipboard. */
function stubClipboard() {
  const writeText = vi.fn(async (_text: string) => {});
  vi.stubGlobal("navigator", { ...navigator, clipboard: { writeText } });
  return writeText;
}

/** Makes the window as narrow as a phone's when `phone`. */
function stubWidth(phone: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches: phone && query.includes("max-width"),
    addEventListener: () => {},
    removeEventListener: () => {},
  }));
}

function renderPage(slug: string | null, selected: string | null = null) {
  return render(
    <QueryClientProvider client={makeQueryClient()}>
      <Workflows slug={slug} selected={selected} onSignedOut={() => {}} />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Workflows on desktop", () => {
  it("draws the first graph that the check passed, with its source row and lanes", async () => {
    stubServer();
    stubWidth(false);
    renderPage(null);

    expect(
      await screen.findByRole("heading", { level: 1, name: "Development flow" }),
    ).toBeVisible();
    expect(screen.getByText("/flows/development/3029fae590f5a628")).toBeVisible();
    expect(screen.getByText("Not checked against the running flow")).toBeVisible();
    expect(screen.getByText(/Two earlier readings in/)).toBeVisible();
    expect(screen.getByRole("button", { name: "Open the flow record" })).toBeVisible();
    const graph = screen.getByRole("region", { name: "Graph" });
    for (const lane of ["Lead", "Owner", "Builder", "Reviewer", "Board"]) {
      expect(within(graph).getByText(lane, { selector: "b" })).toBeVisible();
    }
    const switcher = screen.getByRole("group", { name: "Workflow" });
    expect(within(switcher).getByRole("button", { name: "Development flow" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    // The panel starts on the start node.
    const panel = screen.getByRole("complementary", { name: "Selected" });
    expect(within(panel).getByRole("heading", { name: "changes start" })).toBeVisible();
  });

  it("fades unrelated nodes when a node is selected and fills the panel with its next steps and changes", async () => {
    stubServer();
    stubWidth(false);
    renderPage("development-flow");
    const paused = await screen.findByRole("button", { name: /^Waits: Paused/ });

    await userEvent.click(paused);

    expect(paused).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: /^Step: Build test-first/ })).not.toHaveClass(
      "wf-node-dim",
    );
    expect(screen.getByRole("button", { name: /^Check: Plan names its task/ })).toHaveClass(
      "wf-node-dim",
    );
    const panel = screen.getByRole("complementary", { name: "Selected" });
    expect(within(panel).getByRole("heading", { name: "Paused" })).toBeVisible();
    expect(within(panel).getByRole("button", { name: "Build test-first" })).toBeVisible();
    expect(within(panel).getByText("Here now")).toBeVisible();
    expect(within(panel).getByRole("button", { name: "A paused change" })).toBeVisible();
  });

  it("shows a phase's changes, those on the graph's version first, with a violet version chip", async () => {
    stubServer();
    stubWidth(false);
    renderPage("development-flow", "plan");

    const panel = await screen.findByRole("complementary", { name: "Selected" });
    expect(within(panel).getByRole("heading", { name: "Plan" })).toBeVisible();
    const titles = within(panel)
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(titles.slice(-3)).toEqual([
      "The page shows a sample",
      "A plan on an older flow",
      "Another plan on an older flow",
    ]);
    expect(within(panel).getAllByText("flow 23eb709b")).toHaveLength(2);
    expect(within(panel).getByText(/2 records were skipped/)).toBeVisible();
  });

  it("selects the node whose binding placed the change that the address names", async () => {
    stubServer();
    stubWidth(false);
    renderPage("development-flow", "/changes/sample-paused");

    expect(await screen.findByRole("button", { name: /^Waits: Paused/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("hides the conditional checks when the toggle is on", async () => {
    stubServer();
    stubWidth(false);
    renderPage("development-flow");
    await screen.findByRole("button", { name: /^Waits: Owner signs the mockups/ });

    await userEvent.click(screen.getByRole("checkbox", { name: /Hide checks/ }));

    expect(screen.queryByRole("button", { name: /^Waits: Owner signs the mockups/ })).toBeNull();
    expect(screen.getByRole("button", { name: /^Waits: Owner signs the plan/ })).toBeVisible();
  });

  it("marks every change not on the running version as on an older flow when a newer version runs", async () => {
    stubServer(developmentFlowOnOlderFlow);
    stubWidth(false);
    renderPage("development-flow", "plan");

    const panel = await screen.findByRole("complementary", { name: "Selected" });
    expect(
      screen.getByText(/6 changes are open on this flow, 6 of them on an older version/),
    ).toBeVisible();
    expect(screen.getByText(/The changes plugin now runs/)).toBeVisible();
    expect(screen.getByText("Older than the running flow")).toBeVisible();
    expect(screen.getByText(/open, 3 on an older flow/)).toBeVisible();
    expect(screen.getAllByText(/open, 1 on an older flow/)).toHaveLength(2);
    const graph = screen.getByRole("region", { name: "Graph" });
    const dots = graph.querySelectorAll(".wf-bandlab .wf-dot");
    expect(dots).toHaveLength(5);
    expect(graph.querySelectorAll(".wf-bandlab .wf-dot.wf-dot-old")).toHaveLength(5);
    expect(within(panel).getByText("flow 3029fae5")).toBeVisible();
    expect(within(panel).getAllByText("flow 23eb709b")).toHaveLength(2);
    expect(within(panel).queryByText("this version")).toBeNull();
  });

  it("copies a board get command that quotes a path holding a shell character", async () => {
    const writeText = stubClipboard();
    const odd = structuredClone(developmentFlow);
    const [first] = odd.data.bindings.flatMap((binding) => binding.records);
    if (first === undefined) {
      throw new Error("the fixture places no change");
    }
    first.path = "/changes/a b;rm -rf ~";
    stubServer(odd);
    stubWidth(false);
    renderPage("development-flow", "plan");
    const panel = await screen.findByRole("complementary", { name: "Selected" });

    await userEvent.click(screen.getByRole("button", { name: "Open the flow record" }));
    await userEvent.click(within(panel).getByRole("button", { name: "The page shows a sample" }));

    expect(writeText.mock.calls.map(([text]) => text)).toEqual([
      "board get /flows/development/3029fae590f5a628",
      "board get '/changes/a b;rm -rf ~'",
    ]);
  });

  it("copies nothing for a path holding a control character and says why", async () => {
    const writeText = stubClipboard();
    const odd = structuredClone(developmentFlow);
    const [first] = odd.data.bindings.flatMap((binding) => binding.records);
    if (first === undefined) {
      throw new Error("the fixture places no change");
    }
    first.path = "/changes/a\u001b[201~b";
    stubServer(odd);
    stubWidth(false);
    renderPage("development-flow", "plan");
    const panel = await screen.findByRole("complementary", { name: "Selected" });

    await userEvent.click(within(panel).getByRole("button", { name: "The page shows a sample" }));

    expect(writeText).not.toHaveBeenCalled();
    expect(
      screen.getByText(/holds a control character, so flightdeck does not copy it/, {
        selector: ".wf-cap",
      }),
    ).toBeVisible();
  });

  it("shows the problems and no graph for a record the check rejects", async () => {
    stubServer();
    stubWidth(false);
    renderPage("tangled");

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("The check found 1 problem");
    expect(within(alert).getByText("/edges/2/to")).toBeVisible();
    expect(screen.queryByRole("region", { name: "Graph" })).toBeNull();
  });
});

describe("Workflows on a phone", () => {
  it("shortens each graph's title in the switch", async () => {
    stubServer();
    stubWidth(true);
    renderPage("development-flow");

    const switcher = await screen.findByRole("group", { name: "Workflow" });
    expect(
      within(switcher)
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["broken", "Development", "Small", "Tangled", "Weekly loop"]);
  });

  it("is a run sheet with lane labels and conditional checks hidden at first", async () => {
    stubServer();
    stubWidth(true);
    renderPage("development-flow");

    const plan = await screen.findByRole("button", { name: /^Interview .* and write the plan/ });
    expect(plan).toHaveTextContent("Lead");
    expect(screen.getByRole("checkbox", { name: /Hide checks/ })).toBeChecked();
    expect(screen.queryByRole("button", { name: /signs the mockups/ })).toBeNull();
    expect(screen.queryByRole("region", { name: "Graph" })).toBeNull();
    const approval = screen.getByRole("button", { name: /signs the plan/ });
    expect(approval).toHaveTextContent(/If it fails, back to Interview .* and write the plan/);

    await userEvent.click(approval);

    expect(approval).toHaveAttribute("aria-expanded", "true");
    expect(approval).toHaveTextContent("/phases/0/checks/4");
  });
});
