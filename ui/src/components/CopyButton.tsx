import { Check, Copy } from "lucide-react";
import { useState } from "react";

/** Copies `text` to the clipboard, and says so once it has. */
export function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard.writeText(text).then(
      () => setCopied(true),
      () => setCopied(false),
    );
  };
  return (
    <button
      type="button"
      onClick={copy}
      onBlur={() => setCopied(false)}
      className="text-primary inline-flex items-center gap-1 text-xs hover:underline"
    >
      {copied ? (
        <Check aria-hidden="true" className="size-3.5" />
      ) : (
        <Copy aria-hidden="true" className="size-3.5" />
      )}
      {copied ? "Copied" : label}
    </button>
  );
}
