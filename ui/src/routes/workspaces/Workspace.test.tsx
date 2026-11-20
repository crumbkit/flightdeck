import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeQueryClient } from "@/queryClient";
import { readingList } from "../../../tests/fixtures/workspaces";
import { Workspace } from "./Workspace";

/** Answers every request with `reply`, and records each path. */
function stubServer(reply: Response) {
  const asked = vi.fn();
  vi.stubGlobal("fetch", async (path: string) => {
    asked(path);
    return reply.clone();
  });
  return asked;
}

function renderWorkspace(name: string) {
  return render(
    <QueryClientProvider client={makeQueryClient()}>
      <Workspace name={name} onSignedOut={() => {}} />
    </QueryClientProvider>,
  );
}

/** Matches the one element of a port, whose words sit in two parts. */
function portOf(text: string) {
  return (_: string, element: Element | null) =>
    element?.tagName === "DIV" && element.textContent === text;
}

/** Returns the units table's row of the unit `name`. */
function unitRow(name: string): HTMLElement {
  const table = screen.getByRole("table", { name: "Units" });
  const cell = within(table).getByRole("cell", { name });
  const row = cell.closest("tr");
  if (row === null) {
    throw new Error(`no row for ${name}`);
  }
  return row;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Workspace", () => {
  it("asks for the workspace by its escaped name", async () => {
    const asked = stubServer(Response.json(readingList));
    renderWorkspace("reading-list");

    await screen.findByRole("heading", { level: 1, name: "reading-list" });
    expect(asked).toHaveBeenCalledWith("/api/workspaces/reading-list");
  });

  it("heads the page with the workspace's name and a breadcrumb that ends at local", async () => {
    stubServer(Response.json(readingList));
    renderWorkspace("reading-list");

    expect(await screen.findByRole("heading", { level: 1, name: "reading-list" })).toBeVisible();
    const crumbs = screen.getByRole("navigation", { name: "Breadcrumb" });
    expect(crumbs).toHaveTextContent(/^local$/);
    expect(within(crumbs).getByRole("link", { name: "local" })).toHaveAttribute(
      "href",
      "#/workspaces",
    );
  });

  it("lets a port wrap only between its name and its number, and keeps a unit's state on one line", async () => {
    stubServer(Response.json(readingList));
    renderWorkspace("reading-list");
    await screen.findByRole("table", { name: "Units" });

    const port = within(unitRow("api")).getByText(portOf("READING_API_PORT=41002"));
    expect(port.querySelector("wbr")).not.toBeNull();
    expect(within(port).getByText("READING_API_PORT=")).toHaveClass("whitespace-nowrap");
    expect(within(port).getByText("41002")).toHaveClass("whitespace-nowrap");
    expect(within(unitRow("web")).getByText("running, not ready")).toHaveClass("whitespace-nowrap");
  });

  it("gives each checkout's branch, commit and uncommitted changes", async () => {
    stubServer(Response.json(readingList));
    renderWorkspace("reading-list");

    const checkouts = await screen.findByRole("region", { name: "Checkouts" });
    expect(within(checkouts).getAllByText("reading-list")).toHaveLength(2);
    expect(within(checkouts).getByText("3333333")).toBeVisible();
    expect(within(checkouts).getByText("2 uncommitted changes")).toBeVisible();
    expect(within(checkouts).getByText("no uncommitted changes")).toBeVisible();
  });

  it("lists each unit with its state, ports and route link", async () => {
    stubServer(Response.json(readingList));
    renderWorkspace("reading-list");
    await screen.findByRole("table", { name: "Units" });

    expect(within(unitRow("api")).getByText("ready")).toBeVisible();
    expect(within(unitRow("api")).getByText(portOf("READING_API_PORT=41002"))).toBeVisible();
    const web = within(unitRow("web"));
    expect(web.getByText("running, not ready")).toBeVisible();
    expect(web.getByRole("link", { name: "route" })).toHaveAttribute(
      "href",
      "https://web.reading-list.mxv.localhost",
    );
    expect(within(unitRow("cache")).getByText("exited with 137")).toBeVisible();
    expect(within(unitRow("cache")).queryByRole("link")).toBeNull();
  });

  it("says no workspace is named so, with a link back, when mxv does not know the name", async () => {
    stubServer(
      Response.json(
        {
          schema: "flightdeck/failure/v1",
          code: "flightdeck::workspaces::not_found",
          message: "no workspace is named gone",
        },
        { status: 404 },
      ),
    );
    renderWorkspace("gone");

    expect(await screen.findByText("No workspace is named gone.")).toBeVisible();
    expect(screen.getByRole("link", { name: "Back to the workspaces" })).toHaveAttribute(
      "href",
      "#/workspaces",
    );
  });

  it("says why the units are missing when mxv cannot read the manifest", async () => {
    stubServer(
      Response.json({
        ...readingList,
        data: {
          ...readingList.data,
          units: [],
          manifest_error: "mxv.toml names no unit kind",
          manifest_error_code: "mxv::manifest::invalid",
        },
      }),
    );
    renderWorkspace("reading-list");

    expect(await screen.findByText("mxv.toml names no unit kind")).toBeVisible();
  });
});
