import { cn } from "cn";
import type { Effect } from "@/lib/rail";

/** The colour of each effect's dot and word. */
const inks: Record<Effect, string> = {
  "read-only": "text-read-only",
  additive: "text-additive",
  destructive: "text-destructive",
  "effects unknown": "text-destructive",
};

/** The soft fill behind each effect's pill. */
const fills: Record<Effect, string> = {
  "read-only": "bg-read-only-soft",
  additive: "bg-additive-soft",
  destructive: "bg-destructive-soft",
  "effects unknown": "bg-destructive-soft",
};

type Props = {
  effect: Effect;
  /**
   * True for the dot and the word alone, without the pill behind them,
   * where many badges sit close together, as in the rail.
   */
  quiet?: boolean;
};

/** A command's effect as a coloured dot and its word, never the colour alone. */
export function EffectBadge({ effect, quiet = false }: Props) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 text-xs whitespace-nowrap",
        inks[effect],
        quiet ? "font-medium" : cn("rounded-full px-2 py-0.5 font-semibold", fills[effect]),
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
