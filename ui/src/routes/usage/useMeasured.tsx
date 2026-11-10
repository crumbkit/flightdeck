import { useQuery } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { describeFailure } from "@/api/failure";
import type { Tool } from "@/api/model";
import { type WeightReply, weightQuery } from "@/api/weight";

/**
 * The help pages' measurement for a tab that reads it: the reply, or what
 * the tab shows in its place while flightdeck measures, when the request
 * failed, or when the measurement could not start.
 */
type Measured = { reply: WeightReply; instead: null } | { reply: null; instead: ReactElement };

/**
 * Returns the measurement of `tool`'s help pages that the weight and both
 * tabs share. They read one query, so the help pages run once.
 */
export function useMeasured(tool: Tool): Measured {
  const weight = useQuery(weightQuery());
  if (weight.isPending) {
    return {
      reply: null,
      instead: (
        <p role="status" className="text-muted-foreground m-0 px-4 py-3 text-sm">
          Measuring {tool.name}'s help pages…
        </p>
      ),
    };
  }
  if (weight.isError) {
    return {
      reply: null,
      instead: (
        <p role="alert" className="text-destructive m-0 px-4 py-3 text-sm">
          {describeFailure(weight.error)}
        </p>
      ),
    };
  }
  if (weight.data.problem !== null) {
    return {
      reply: null,
      instead: (
        <p role="alert" className="text-destructive m-0 px-4 py-3 text-sm">
          flightdeck could not measure help pages: {weight.data.problem}
        </p>
      ),
    };
  }
  return { reply: weight.data, instead: null };
}
