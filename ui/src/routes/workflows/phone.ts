import { useSyncExternalStore } from "react";

/** The widest window that shows the Workflows page as a phone's run sheet. */
const phoneQuery = "(max-width: 639px)";

/** Returns whether the window is as narrow as a phone's, and draws again when that changes. */
export function usePhone(): boolean {
  return useSyncExternalStore(
    (changed) => {
      const media = window.matchMedia?.(phoneQuery);
      media?.addEventListener("change", changed);
      return () => media?.removeEventListener("change", changed);
    },
    () => window.matchMedia?.(phoneQuery).matches ?? false,
  );
}
