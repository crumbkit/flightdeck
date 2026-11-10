import { queryOptions } from "@tanstack/react-query";
import { z } from "@/api/zod";
import { postJson } from "./client";
import { CommandPath, type Link } from "./model";

/**
 * The choices of an argument as `POST /api/choices` sends them: the value
 * of each row's `id`, labelled by the cells of the columns that `labels`
 * names, and whether the source had more rows than the reply holds.
 */
const ChoicesReply = z.object({
  source: CommandPath,
  column: z.string(),
  labels: z.array(z.string()),
  choices: z.array(z.object({ value: z.string(), label: z.array(z.unknown()) })),
  truncated: z.boolean(),
});
export type ChoicesReply = z.infer<typeof ChoicesReply>;

/**
 * The choices query of `link` in `directory`, or in the directory
 * flightdeck started in when that is null (APP-10). The server runs the
 * link's source once for each directory, because a listing can depend on
 * it, and the page keeps the choices until it reloads.
 */
export function choicesQuery(link: Link, directory: string | null) {
  return queryOptions({
    queryKey: ["choices", link.source, link.target, link.argument, directory],
    queryFn: ({ signal }) =>
      postJson(
        "/api/choices",
        {
          source: link.source,
          target: link.target,
          argument: link.argument,
          ...(directory === null ? {} : { directory }),
        },
        ChoicesReply,
        signal,
      ),
    staleTime: Number.POSITIVE_INFINITY,
    gcTime: Number.POSITIVE_INFINITY,
    retry: false,
  });
}
