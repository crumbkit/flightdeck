// Reads the compliance document from the app's source, once per source and
// again on each reload, and keeps the last document it read while a later
// read fails, so a page never empties because the network dropped.
import { type ComplianceDocument, InvalidDocument } from "@flightdeck/headless/document";
import { type ComplianceSource, readDocument } from "@flightdeck/headless/source";
import { useCallback, useEffect, useState } from "react";

/** What reading the document has given so far. */
export interface ComplianceDocumentState {
  /** The newest document read, or null before the first read succeeds. */
  readonly document: ComplianceDocument | null;
  /** Why the newest read failed: SourceFailed or InvalidDocument; null after a read succeeds. */
  readonly error: Error | null;
  /** Whether a read is under way. */
  readonly loading: boolean;
  /** Reads the document again. */
  readonly reload: () => void;
}

/**
 * Returns the document `source` reads. A read starts when the source
 * changes and on each reload, and the read before it is aborted. A failed
 * read keeps the last document and reports its error.
 *
 * `source` must be stable, such as one made at module scope or kept with
 * useMemo, because each new source object starts a new read; an object
 * written inline in the render would read again on every render.
 */
export function useComplianceDocument(source: ComplianceSource): ComplianceDocumentState {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<Omit<ComplianceDocumentState, "reload">>({
    document: null,
    error: null,
    loading: true,
  });
  useEffect(() => {
    // `attempt` counts reloads; reading it here makes each reload a new read.
    void attempt;
    const controller = new AbortController();
    setState((was) => ({ ...was, loading: true }));
    readDocument(source, controller.signal).then(
      (document) => {
        if (!controller.signal.aborted) setState({ document, error: null, loading: false });
      },
      (error: unknown) => {
        if (controller.signal.aborted) return;
        const failure = error instanceof Error ? error : new InvalidDocument("", String(error));
        setState((was) => ({ document: was.document, error: failure, loading: false }));
      },
    );
    return () => controller.abort();
  }, [source, attempt]);
  const reload = useCallback(() => setAttempt((count) => count + 1), []);
  return { ...state, reload };
}
