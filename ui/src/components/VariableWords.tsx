import { Fragment, useContext } from "react";
import type { Tool } from "@/api/model";
import { OpenVariable } from "@/lib/openVariable";
import { partsOf } from "@/lib/variableParts";

type Props = {
  text: string;
  tool: Tool;
};

/**
 * A sentence of a failure, with the name of each variable that the tool
 * declares drawn as a link that opens the environment view on its row. On
 * a page without the environment view the sentence is plain text.
 */
export function VariableWords({ text, tool }: Props) {
  const open = useContext(OpenVariable);
  const names = (tool.environment ?? []).map((each) => each.name);
  if (open === null || names.length === 0) {
    return text;
  }
  return partsOf(text, names).map((part, index) =>
    part.variable ? (
      <button
        // A sentence's parts never change order, and a name may repeat.
        // biome-ignore lint/suspicious/noArrayIndexKey: a sentence's parts are fixed
        key={index}
        type="button"
        title={`Show ${part.text} in the environment view`}
        onClick={() => open(part.text)}
        className="text-primary rounded-xs font-mono underline decoration-dotted underline-offset-3 hover:decoration-solid"
      >
        {part.text}
      </button>
    ) : (
      // biome-ignore lint/suspicious/noArrayIndexKey: a sentence's parts are fixed
      <Fragment key={index}>{part.text}</Fragment>
    ),
  );
}
