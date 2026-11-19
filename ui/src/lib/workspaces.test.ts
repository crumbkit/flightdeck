import { describe, expect, it } from "vitest";
import type { UnitDetail, WorkspaceRow } from "@/api/workspaces";
import { addressesOf, dotsOf, unitsLine, unitTone, unitWord } from "./workspaces";

/** A workspace row with two running units, to change one field at a time. */
const row: WorkspaceRow = {
  name: "reading-list",
  branch: "reading-list",
  checkouts: ["reading", "shelf"],
  ports: { web: 41001, api: 41002 },
  problem: null,
  routes: [{ route: "web", url: "https://web.reading-list.mxv.localhost", unit: "web" }],
  declared: 3,
  running: 2,
  pinned: false,
  units: [
    { name: "api", state: "running" },
    { name: "web", state: "running" },
    { name: "cache", state: "exited" },
  ],
  status_error: null,
};

/** A running, ready unit, to change one field at a time. */
const unit: UnitDetail = {
  name: "api",
  kind: "process",
  state: "running",
  ready: true,
  ports: ["READING_API_PORT=41002"],
  code: null,
  signal: null,
  reason: null,
  routes: [],
};

describe("unitsLine", () => {
  it("counts the running units out of the declared ones", () => {
    expect(unitsLine(row)).toBe("2 of 3 running");
  });

  it("says stopped when units are declared and none runs", () => {
    expect(unitsLine({ ...row, running: 0, declared: 1 })).toBe("stopped");
  });

  it("says none declared for a workspace without units", () => {
    expect(unitsLine({ ...row, running: 0, declared: 0, units: [] })).toBe("none declared");
  });

  it("says units unknown with mxv's code when its status failed", () => {
    const failed = {
      ...row,
      running: null,
      declared: null,
      units: null,
      status_error: { code: "mxv::manifest::invalid", message: "bad manifest" },
    };

    expect(unitsLine(failed)).toBe("units unknown · mxv::manifest::invalid");
    expect(unitsLine({ ...failed, status_error: { code: null, message: "timed out" } })).toBe(
      "units unknown",
    );
  });
});

describe("dotsOf", () => {
  it("draws a running unit filled, an exited one as failed and any other as empty", () => {
    const units = [...(row.units ?? []), { name: "db", state: "stopped" }];

    expect(dotsOf({ ...row, units })).toEqual(["on", "on", "bad", "off"]);
  });

  it("draws no dot for a workspace whose units are unknown", () => {
    expect(dotsOf({ ...row, units: null })).toEqual([]);
  });
});

describe("addressesOf", () => {
  it("gives the proxy's routes by host name, linked to their URL", () => {
    expect(addressesOf(row)).toEqual([
      { text: "web.reading-list.mxv.localhost", href: "https://web.reading-list.mxv.localhost" },
    ]);
  });

  it("falls back to each host port on 127.0.0.1 in name order when the workspace has no route", () => {
    expect(addressesOf({ ...row, routes: [] })).toEqual([
      { text: "127.0.0.1:41002", href: null },
      { text: "127.0.0.1:41001", href: null },
    ]);
  });

  it("gives nothing for a workspace with neither", () => {
    expect(addressesOf({ ...row, routes: [], ports: {} })).toEqual([]);
  });
});

describe("unitWord and unitTone", () => {
  it("call a running unit that passed its readiness check ready", () => {
    expect(unitWord(unit)).toBe("ready");
    expect(unitTone(unit)).toBe("on");
  });

  it("call a running unit that is not ready yet running, not ready", () => {
    expect(unitWord({ ...unit, ready: false })).toBe("running, not ready");
  });

  it("give an exited unit its exit code or signal and mark it failed", () => {
    const exited = { ...unit, state: "exited", ready: null, code: 137 };

    expect(unitWord(exited)).toBe("exited with 137");
    expect(unitWord({ ...exited, code: null, signal: 15 })).toBe("exited on signal 15");
    expect(unitTone(exited)).toBe("bad");
  });

  it("give any other state as mxv words it", () => {
    const stopped = { ...unit, state: "stopped", ready: null };

    expect(unitWord(stopped)).toBe("stopped");
    expect(unitTone(stopped)).toBe("off");
  });
});
