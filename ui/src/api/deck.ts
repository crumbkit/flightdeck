import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { getJson } from "./client";

/**
 * The reply of `GET /api/deck`: whether flightdeck started with a tool,
 * and whether the board program answered, so the rail shows Workflows.
 */
const DeckReply = z.object({ tool: z.boolean(), board: z.boolean() });
export type DeckReply = z.infer<typeof DeckReply>;

/** The deck query (APP-10). What the server holds never changes while it runs. */
export function deckQuery() {
  return queryOptions({
    queryKey: ["deck"],
    queryFn: ({ signal }) => getJson("/api/deck", DeckReply, signal),
    staleTime: Number.POSITIVE_INFINITY,
  });
}
