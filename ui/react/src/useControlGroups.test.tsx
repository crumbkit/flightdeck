import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureDocument, useMemoryAddress } from "./testing.tsx";
import { useControlGroups } from "./useControlGroups.ts";

const compliance = fixtureDocument();

function Page() {
  const address = useMemoryAddress();
  const page = useControlGroups(compliance, address);
  return (
    <div>
      <h1>{page.view.headline}</h1>
      {page.view.groups.map((group) => (
        <section key={group.id}>
          <button {...page.getGroupToggleProps(group)}>{group.title}</button>
          <ul {...page.getGroupRegionProps(group)}>
            {group.members.map((member) => (
              <li key={member.id} {...page.getMemberRowProps(group, member)}>
                {member.short}
                {member.alsoIn.map((other) => (
                  <a key={other.id} {...page.getAlsoInProps(member, other)}>
                    {`${other.title} ↔`}
                  </a>
                ))}
              </li>
            ))}
          </ul>
        </section>
      ))}
      <output>{address.search}</output>
    </div>
  );
}

describe("useControlGroups", () => {
  it("collapses the first group when the reader closes it", async () => {
    render(<Page />);
    const rust = screen.getByRole("button", { name: "Rust" });
    await userEvent.click(rust);
    expect(screen.getByRole("status")).toHaveTextContent("?group=none");
    expect(rust).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByRole("button", { name: "Server" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("expands the first group, and moves the expanded group into the address on a press", async () => {
    render(<Page />);
    const rust = screen.getByRole("button", { name: "Rust" });
    const server = screen.getByRole("button", { name: "Server" });
    expect(rust).toHaveAttribute("aria-expanded", "true");
    expect(server).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(server);
    expect(screen.getByRole("status")).toHaveTextContent("?group=%2Frules%2Fgroups%2Fserver");
    expect(server).toHaveAttribute("aria-expanded", "true");
    expect(rust).toHaveAttribute("aria-expanded", "false");
  });

  it("opens the other group with the control marked and focused when the reader follows it", async () => {
    render(<Page />);
    await userEvent.click(screen.getByRole("link", { name: "Server ↔" }));
    expect(screen.getByRole("status")).toHaveTextContent(
      "?group=%2Frules%2Fgroups%2Fserver&rule=%2Frules%2Frust%2Fserver",
    );
    const marked = document.querySelector('[aria-current="true"]');
    expect(marked?.textContent).toMatch(/^server/);
    expect(marked).toHaveFocus();
    expect(screen.getByRole("button", { name: "Server" })).toHaveAttribute("aria-expanded", "true");
  });
});
