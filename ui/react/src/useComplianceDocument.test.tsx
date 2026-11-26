import type { ComplianceSource } from "@flightdeck/headless/source";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { fixtureJson } from "./testing.tsx";
import { useComplianceDocument } from "./useComplianceDocument.ts";

describe("useComplianceDocument", () => {
  it("reads the document from the app's source", async () => {
    const source: ComplianceSource = { read: async () => fixtureJson };
    const { result } = renderHook(() => useComplianceDocument(source));
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.document?.title).toBe("Compliance"));
    expect(result.current.error).toBeNull();
  });

  it("keeps the last document when a reload fails, and reports why", async () => {
    let fail = false;
    const source: ComplianceSource = {
      read: async () => {
        if (fail) throw new Error("offline");
        return fixtureJson;
      },
    };
    const { result } = renderHook(() => useComplianceDocument(source));
    await waitFor(() => expect(result.current.document).not.toBeNull());
    fail = true;
    act(() => result.current.reload());
    await waitFor(() => expect(result.current.error?.message).toMatch(/could not be read/));
    expect(result.current.document?.title).toBe("Compliance");
    expect(result.current.error?.cause).toEqual(new Error("offline"));
  });

  it("aborts the read when the page leaves", async () => {
    const signals: AbortSignal[] = [];
    const source: ComplianceSource = {
      read: (signal) => {
        if (signal instanceof AbortSignal) signals.push(signal);
        return new Promise(() => undefined);
      },
    };
    const { unmount } = renderHook(() => useComplianceDocument(source));
    unmount();
    expect(signals.map((signal) => signal.aborted)).toEqual([true]);
  });
});
