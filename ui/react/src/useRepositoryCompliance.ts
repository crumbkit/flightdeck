// One repository's compliance hook (artboards 47, 55 and 56). The selected
// rule lives in the page address. Choosing a rule moves focus to its detail,
// so a keyboard or screen reader user lands on what they opened.

import type { ComplianceDocument } from "@crumbkit/flightdeck-headless/document";
import {
  type RepositoryView,
  repositoryAddress,
  repositoryView,
} from "@crumbkit/flightdeck-headless/repository";
import { type AddressSource, useAddressState } from "@crumbkit/ui/react";
import { type RefCallback, useCallback, useEffect, useId, useMemo, useRef } from "react";

/** The props of the button that selects a rule. */
export interface RuleButtonProps {
  readonly type: "button";
  readonly "aria-pressed": boolean;
  readonly "aria-controls": string;
  readonly onClick: () => void;
}

/** The props of the selected rule's detail. */
export interface DetailProps {
  readonly id: string;
  readonly role: "region";
  readonly "aria-label": string;
  readonly tabIndex: -1;
  readonly ref: RefCallback<HTMLElement>;
}

/** Returns the page of the repository `name`, with the selected rule held in `address`; null when the document has no such repository. */
export function useRepositoryCompliance(
  document: ComplianceDocument,
  name: string,
  address: AddressSource,
) {
  const [value, change] = useAddressState(repositoryAddress, address);
  const view: RepositoryView | null = useMemo(
    () => repositoryView(document, name, value),
    [document, name, value],
  );
  const detailId = useId();
  const detail = useRef<HTMLElement | null>(null);
  // Set when the reader chooses a rule, so focus moves only then and not
  // when the page first opens on the address's rule.
  const chosen = useRef(false);
  const selected = view?.selected?.key ?? null;
  useEffect(() => {
    if (chosen.current && selected !== null) {
      chosen.current = false;
      detail.current?.focus();
    }
  }, [selected]);
  const detailRef = useCallback((element: HTMLElement | null) => {
    detail.current = element;
  }, []);
  return {
    view,
    getRuleButtonProps: (id: string): RuleButtonProps => ({
      type: "button",
      "aria-pressed": view?.selected?.id === id,
      "aria-controls": detailId,
      onClick: () => {
        if (view?.selected?.id === id) {
          detail.current?.focus();
          return;
        }
        chosen.current = true;
        change({ rule: id });
      },
    }),
    getDetailProps: (): DetailProps => ({
      id: detailId,
      role: "region",
      "aria-label": view?.selected?.label ?? "",
      tabIndex: -1,
      ref: detailRef,
    }),
  };
}
