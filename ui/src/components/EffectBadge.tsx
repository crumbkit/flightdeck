import { cn } from "cn";
import type { Effect } from "@/lib/rail";

const tones: Record<Effect, string> = {
  "read-only": "bg-read-only-soft text-read-only",
  additive: "bg-additive-soft text-additive",
  destructive: "bg-destructive-soft text-destructive",
  "effects unknown": "bg-destructive-soft text-destructive",
};

/** A command's effect as a coloured dot and its word, never the colour alone. */
export function EffectBadge({ effect }: { effect: Effect }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-xs font-semibold whitespace-nowrap",
        tones[effect],
      )}
    >
      <span
        aria-hidden="true"
        className={cn(
          "size-1.5 rounded-full",
          effect === "effects unknown" ? "border border-current" : "bg-current",
        )}
      />
      {effect}
    </span>
  );
}
