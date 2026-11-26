import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureDocument, useMemoryAddress } from "./testing.tsx";
import { useCompliance } from "./useCompliance.ts";

const compliance = fixtureDocument();

function Page({ onAddress }: { onAddress?: (search: string) => void }) {
  const address = useMemoryAddress();
  const page = useCompliance(compliance, {
    search: address.search,
    navigate: (search) => {
      onAddress?.(search);
      address.navigate(search);
    },
  });
  return (
    <div>
      <h1>{page.view.headline}</h1>
      {page.showOptions.map(({ option, label }) => (
        <button key={option} {...page.getShowProps(option)}>
          {label}
        </button>
      ))}
      {page.view.tiles.map((tile) => {
        const link = page.getTileProps(tile);
        return link === null ? (
          <span key={tile.key}>{tile.label}</span>
        ) : (
          <a key={tile.key} {...link}>
            {tile.label}
          </a>
        );
      })}
      <table {...page.getGridProps()}>
        <tbody>
          {page.rows.map((row, at) => (
            <tr key={row.key} {...page.getRowProps()}>
              <td {...page.getCellProps(at, 0)}>{row.name}</td>
              <td {...page.getCellProps(at, 1)}>{row.checker}</td>
              {row.cells.map((cell, column) => (
                <td
                  key={cell.repository}
                  {...page.getCellProps(at, column + 2)}
                  aria-label={cell.spoken}
                >
                  {cell.word}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <button {...page.getWrittenToggleProps()}>{page.view.written.toggle}</button>
      <div {...page.getListProps("failing")}>Failing · {page.view.failing.count}</div>
    </div>
  );
}

describe("useCompliance", () => {
  it("presses the Show chip the address holds and writes a press to the address", async () => {
    const seen: string[] = [];
    render(<Page onAddress={(search) => seen.push(search)} />);
    const every = screen.getByRole("button", { name: "Every state" });
    const notCurrent = screen.getByRole("button", { name: "Not current" });
    expect(every).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(notCurrent);
    expect(seen).toEqual(["?show=not_current"]);
    expect(notCurrent).toHaveAttribute("aria-pressed", "true");
    expect(every).toHaveAttribute("aria-pressed", "false");
  });

  it("links each tile with a list to that list", () => {
    render(<Page />);
    const failing = screen.getByRole("link", { name: "Failing" });
    const list = screen.getByText("Failing · 6");
    expect(failing.getAttribute("href")).toBe(`#${list.id}`);
    expect(screen.queryByRole("link", { name: "Current" })).toBeNull();
  });

  it("makes the matrix a grid with one tab stop that the arrow keys move", async () => {
    render(<Page />);
    expect(screen.getByRole("grid")).toBeInTheDocument();
    const cells = screen.getAllByRole("gridcell");
    expect(cells.filter((cell) => cell.tabIndex === 0)).toHaveLength(1);
    act(() => cells[0]?.focus());
    expect(cells[0]).toHaveFocus();
    await userEvent.keyboard("{ArrowRight}{ArrowRight}{ArrowRight}");
    expect(document.activeElement).toHaveAccessibleName(
      "owner/api, Keep each Rust file under 60 bytes: failing, 1 over",
    );
  });

  it("opens the written rules into the grid when their button is pressed", async () => {
    render(<Page />);
    const toggle = screen.getByRole("button", { name: "Show 4 written rules" });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.getAllByRole("row")).toHaveLength(4);
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getAllByRole("row")).toHaveLength(8);
  });
});
