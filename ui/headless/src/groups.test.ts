import { describe, expect, it } from "vitest";
import { groupsAddress, groupsView } from "./groups.ts";
import { fixtureDocument } from "./testDocuments.ts";

describe("groupsView of the fixture (artboard 58)", () => {
  const view = groupsView(fixtureDocument(), groupsAddress.parse(""));

  it("says which groups fail and which also drift", () => {
    expect(view.headline).toBe("Each group has a failing control, and Rust and Server also drift");
  });

  it("rolls each group up over the repositories, worst first, with drift after failing", () => {
    expect(
      view.groups.map((group) => [group.title, group.controls, group.rollUp, group.word]),
    ).toEqual([
      ["Rust", "3 controls", "1 failing · 1 drifting · 1 no report · 1 current", "failing"],
      ["Server", "2 controls", "3 failing · 1 no report", "failing"],
    ]);
  });

  it("rolls each control up and names the other groups it is in", () => {
    const rust = view.groups[0];
    expect(rust?.members.map((member) => [member.short, member.rollUp, member.word])).toEqual([
      ["file-size", "1 failing · 1 no report · 2 current", "failing"],
      ["server", "1 drifting · 1 no report · 2 written", "drifts"],
      ["rust", "1 failing · 1 no report · 2 current", "failing"],
    ]);
    expect(rust?.members[1]?.alsoIn).toEqual([
      {
        id: "/rules/groups/server",
        title: "Server",
        search: "?group=%2Frules%2Fgroups%2Fserver&rule=%2Frules%2Frust%2Fserver",
      },
    ]);
  });

  it("counts a control in two groups once", () => {
    expect(view.total).toBe(4);
  });

  it("opens the first group when the address names none, and the named one otherwise", () => {
    expect(view.groups.map((group) => group.expanded)).toEqual([true, false]);
    const server = groupsView(
      fixtureDocument(),
      groupsAddress.parse("?group=%2Frules%2Fgroups%2Fserver&rule=%2Frules%2Frust%2Fserver"),
    );
    expect(server.groups.map((group) => group.expanded)).toEqual([false, true]);
    expect(server.groups[1]?.members.map((member) => member.marked)).toEqual([false, true]);
    expect(server.marked).toBe("/rules/rust/server");
  });

  it("collapses the first group when the reader closes it, and opens no other in its place", () => {
    const rust = view.groups[0];
    const closed = groupsView(fixtureDocument(), { group: rust?.toggleTo ?? null, rule: null });
    expect(closed.groups.map((group) => group.expanded)).toEqual([false, false]);
  });

  it("collapses another group when the reader closes it", () => {
    const server = groupsView(
      fixtureDocument(),
      groupsAddress.parse("?group=%2Frules%2Fgroups%2Fserver"),
    );
    const closed = groupsView(fixtureDocument(), {
      group: server.groups[1]?.toggleTo ?? null,
      rule: null,
    });
    expect(closed.groups.map((group) => group.expanded)).toEqual([false, false]);
    const reopened = groupsView(fixtureDocument(), {
      group: closed.groups[1]?.toggleTo ?? null,
      rule: null,
    });
    expect(reopened.groups.map((group) => group.expanded)).toEqual([false, true]);
  });

  it("gives each group and control a spoken name", () => {
    expect(view.groups[0]?.spoken).toBe(
      "Rust, 3 controls: failing, 1 failing · 1 drifting · 1 no report · 1 current",
    );
    expect(view.groups[0]?.members[0]?.spoken).toBe(
      "file-size: failing, 1 failing · 1 no report · 2 current",
    );
  });
});
