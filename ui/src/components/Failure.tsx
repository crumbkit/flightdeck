import { RotateCw } from "lucide-react";
import { describeFailure } from "@/api/failure";
import { Button } from "@/components/ui/button";

/** The one way the page shows a failed request, with a way to try again (APP-13). */
export function Failure({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  return (
    <div role="alert" className="m-6 flex max-w-xl flex-col items-start gap-3">
      <p>{describeFailure(error)}</p>
      <Button variant="outline" onClick={onRetry}>
        <RotateCw aria-hidden="true" />
        Try again
      </Button>
    </div>
  );
}
