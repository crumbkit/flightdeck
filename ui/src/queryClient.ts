import { QueryClient } from "@tanstack/react-query";
import { ApiError } from "@/api/client";

/** The page's query defaults (APP-12). */
export function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        // A refused or malformed answer does not change on a retry, so only
        // a network failure is retried, and at most twice.
        retry: (failureCount, error) =>
          failureCount < 2 && error instanceof ApiError && error.failure.kind === "network",
      },
    },
  });
}
