// A button that shows or hides a region, such as "Show 11 written rules",
// with the region's id and the button's aria-expanded tied together.
import { useId, useState } from "react";

/** The props of the button that shows or hides a region. */
export interface DisclosureButtonProps {
  readonly type: "button";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly onClick: () => void;
}

/** The props of the region a disclosure shows or hides. */
export interface DisclosureRegionProps {
  readonly id: string;
  readonly hidden: boolean;
}

/** Returns whether a region is open, and prop getters for its button and the region. */
export function useDisclosure(initiallyOpen = false) {
  const id = useId();
  const [open, setOpen] = useState(initiallyOpen);
  return {
    open,
    setOpen,
    getButtonProps: (): DisclosureButtonProps => ({
      type: "button",
      "aria-expanded": open,
      "aria-controls": id,
      onClick: () => setOpen((was) => !was),
    }),
    getRegionProps: (): DisclosureRegionProps => ({ id, hidden: !open }),
  };
}
