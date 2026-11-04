import { cn } from "cn";
import type { Effect } from "@/lib/rail";

/** The colour of each effect's word. */
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
   * True for the word alone, without the pill behind it, where many
   * badges sit close together, as in the rail.
   */
  quiet?: boolean;
};

/** A command's effect as its word in its colour, so the colour never stands alone. */
export function EffectBadge({ effect, quiet = false }: Props) {
  return (
    <span
      className={cn(
        "inline-flex items-center text-xs whitespace-nowrap",
        inks[effect],
        quiet ? "font-medium" : cn("rounded-pill px-2 py-0.5 font-semibold", fills[effect]),
      )}
    >
      {effect}
    </span>
  );
}
