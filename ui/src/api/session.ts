import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { ApiError, hasToken, keepToken, postJson } from "./client";

const Exchanged = z.object({ token: z.string().min(1) });

/** Whether the page may call the API. */
export type Session = "signed-in" | "signed-out";

/**
 * Signs the page in. A launch code is exchanged once at `POST /api/session`
 * for the token, which the client then sends on every request. Without a
 * code, a token kept from before a reload signs the page in. A refused
 * code, or no code and no token, leaves the page signed out.
 */
async function signIn(code: string | null): Promise<Session> {
  if (code !== null) {
    try {
      // The exchange takes no abort signal from TanStack Query, which would
      // cancel it when React mounts the page twice in StrictMode. The server
      // may already have used the code by then, and a second exchange would
      // be refused.
      const never = new AbortController().signal;
      const { token } = await postJson("/api/session", { code }, Exchanged, never);
      keepToken(token);
      return "signed-in";
    } catch (error) {
      if (!(error instanceof ApiError && error.failure.kind === "status")) {
        throw error;
      }
    }
  }
  return hasToken() ? "signed-in" : "signed-out";
}

/**
 * The sign-in query. Its key leaves out the launch code that its `queryFn`
 * reads, because the code is a secret and the page signs in once per load,
 * so no second code ever shares the key.
 */
export function sessionQuery(code: string | null) {
  return queryOptions({
    queryKey: ["session"],
    queryFn: () => signIn(code),
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
}
