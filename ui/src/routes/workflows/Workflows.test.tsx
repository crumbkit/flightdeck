import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeQueryClient } from "@/queryClient";
import { developmentFlow, tangledFlow, workflowsList } from "../../../tests/fixtures/workflows";
import Workflows from "./Workflows";

/** Answers the workflows routes with the fake board's documents. */
function stubServer() {
  vi.stubGlobal("fetch", async (path: string) => {
    if (path === "/api/workflows") {
      return Response.json(workflowsList);
    }
    if (path === "/api/workflows/tangled") {
      return Response.json(tangledFlow);
    }
    return Response.json(developmentFlow);
  });
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
