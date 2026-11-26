// One control's hook (artboard 57). It returns the control's view and the
// disclosure of the repositories it does not apply to.
import { type ControlView, controlView } from "@crumbkit/flightdeck-headless/control";
import type { ComplianceDocument } from "@crumbkit/flightdeck-headless/document";
import { useMemo } from "react";
import { useDisclosure } from "./disclosure.ts";

/** Returns the page of the control `rule`; its view is null when the document holds no such rule. */
export function useControl(document: ComplianceDocument, rule: string) {
  const view: ControlView | null = useMemo(() => controlView(document, rule), [document, rule]);
  const notApplied = useDisclosure();
  return {
    view,
    notAppliedOpen: notApplied.open,
    getNotAppliedToggleProps: notApplied.getButtonProps,
    getNotAppliedRegionProps: notApplied.getRegionProps,
  };
}
