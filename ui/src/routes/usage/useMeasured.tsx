import { useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCw } from "lucide-react";
import type { ReactElement, ReactNode } from "react";
import { describeFailure } from "@/api/failure";
import type { Tool } from "@/api/model";
import { type WeightReply, weightQuery } from "@/api/weight";
import { Button } from "@/components/ui/button";
import { counted } from "@/lib/words";

/**
 * The help pages' measurement for a tab that reads it: the reply, with a
 * notice above the tab's table when the measurement stopped before some
 * pages, or what the tab shows in its place while flightdeck measures,
 * when the request failed, or when the measurement could not start.
 */
type Measured =
  | { reply: WeightReply; notice: ReactElement | null; instead: null }
  | { reply: null; notice: null; instead: ReactElement };

/**
 * Returns the measurement of `tool`'s help pages that the weight and both
 * tabs share. They read one query, so the help pages run once. After a
 * failed request, a measurement with a problem or a partial measurement,
 * the tab offers to measure again.
 */
export function useMeasured(tool: Tool): Measured {
  const weight = useQuery(weightQuery());
  if (weight.isPending) {
    return {
      reply: null,
      notice: null,
      instead: (
        <p role="status" className="text-muted-foreground m-0 px-4 py-3 text-sm">
          Measuring {tool.name}'s help pages…
        </p>
      ),
    };
  }
  if (weight.isError) {
    return { reply: null, notice: null, instead: <Alert>{describeFailure(weight.error)}</Alert> };
  }
  if (weight.data.problem !== null) {
    return {
      reply: null,
      notice: null,
      instead: <Alert>flightdeck could not measure help pages: {weight.data.problem}</Alert>,
    };
  }
  const notRun = weight.data.pages.filter((page) => page.failure === "not_run").length;
  return {
    reply: weight.data,
    notice:
      notRun === 0 ? null : (
        <div className="mb-2.5 flex flex-wrap items-center gap-x-3 gap-y-2">
          <p role="status" className="text-muted-foreground m-0 text-sm">
            flightdeck stopped measuring before {counted(notRun, "help page")}, at its 30 s limit or
            its own stop.
          </p>
          <MeasureAgain />
        </div>
      ),
    instead: null,
  };
}

/** A failure line in place of the tab, with the button to measure again after it. */
function Alert({ children }: { children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3">
      <p role="alert" className="text-destructive m-0 text-sm">
        {children}
      </p>
      <MeasureAgain />
    </div>
  );
}

/**
 * The button that asks the server to measure the help pages again. It
 * returns the weight query to pending, so the tab shows its measuring line,
 * and fetches a new reply with `?again=1`. Prefetching never throws, and a
 * failure lands in the query, where the tab shows it.
 */
function MeasureAgain() {
  const queryClient = useQueryClient();
  const measureAgain = () => {
    queryClient.getQueryCache().find({ queryKey: weightQuery().queryKey })?.reset();
    void queryClient.prefetchQuery(weightQuery({ again: true }));
  };
  return (
    <Button variant="outline" size="sm" onClick={measureAgain}>
      <RotateCw aria-hidden="true" />
      Measure again
    </Button>
  );
}
