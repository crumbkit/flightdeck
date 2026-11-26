import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureDocument, useMemoryAddress } from "./testing.tsx";
import { useRepositoryCompliance } from "./useRepositoryCompliance.ts";

const compliance = fixtureDocument();

function Page({ initial = "" }: { initial?: string }) {
  const address = useMemoryAddress(initial);
  const page = useRepositoryCompliance(compliance, "owner/api", address);
  if (page.view === null) return <p>No such repository</p>;
  return (
    <div>
      <h1>{page.view.headline}</h1>
      {page.view.measured.rows.map((row) => (
        <button key={row.key} {...page.getRuleButtonProps(row.id)}>
          {row.name}
        </button>
      ))}
      <section {...page.getDetailProps()}>
        {page.view.selected?.fields.map((field) => (
          <p key={field.term}>
            {field.term}: {field.value}
          </p>
        ))}
      </section>
      <output>{address.search}</output>
    </div>
  );
}

describe("useRepositoryCompliance", () => {
  it("presses the selected rule's button and names its detail by it", () => {
    render(<Page />);
    expect(
      screen.getByRole("button", { name: "Keep each Rust file under 60 bytes" }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      screen.getByRole("region", { name: "Failing · Keep each Rust file under 60 bytes" }),
    ).toBeInTheDocument();
  });

  it("writes a chosen rule to the address and moves focus to its detail", async () => {
    render(<Page />);
    const pin = screen.getByRole("button", { name: "Pin Rust 1.99 or later" });
    await userEvent.click(pin);
    expect(screen.getByRole("status")).toHaveTextContent("?rule=%2Frules%2Ftoolchains%2Frust");
    expect(pin).toHaveAttribute("aria-pressed", "true");
    const detail = screen.getByRole("region", { name: "Failing · Pin Rust 1.99 or later" });
    expect(detail).toHaveFocus();
    expect(pin).toHaveAttribute("aria-controls", detail.id);
  });

  it("leaves focus where it is when the page opens on the address's rule", () => {
    render(<Page initial="?rule=%2Frules%2Ftoolchains%2Frust" />);
    expect(
      screen.getByRole("region", { name: "Failing · Pin Rust 1.99 or later" }),
    ).not.toHaveFocus();
  });
});
