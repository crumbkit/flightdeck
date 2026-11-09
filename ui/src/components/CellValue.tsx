import { ExternalLink } from "lucide-react";
import { durationWords } from "@/lib/duration";

/**
 * Returns true when all of `text` is an absolute http or https URL. The
 * pattern refuses what `URL` would accept, such as `https:shelf.example`
 * without its slashes, a URL with words after it, extra slashes or
 * backslashes after the scheme, and invisible control or format characters
 * such as U+202E. `URL` refuses the rest, such as `https://%`.
 */
function isWebUrl(text: string): boolean {
  return /^https?:\/\/[^\s\p{Cc}\p{Cf}/\\][^\s\p{Cc}\p{Cf}]*$/iu.test(text) && URL.canParse(text);
}

/**
 * One value of a row as the page shows it. A duration shows its words beside
 * the ISO value the tool printed, such as "1 h 40 min" beside PT1H40M. A
 * value that is wholly an http or https URL is a link that opens in a new
 * tab. No other scheme becomes a link, so a tool's output cannot put a
 * `javascript:` address on the page.
 */
export function CellValue({ value }: { value: unknown }) {
  if (typeof value === "string") {
    if (isWebUrl(value)) {
      return (
        <a
          href={value}
          target="_blank"
          rel="noopener noreferrer"
          className="text-primary font-mono wrap-break-word underline underline-offset-2"
        >
          {value}
          <ExternalLink aria-hidden="true" className="ml-1 inline size-3.5 align-text-bottom" />
        </a>
      );
    }
    const words = durationWords(value);
    return words === null ? (
      value
    ) : (
      <span className="whitespace-nowrap">
        {words} <code className="text-faint-foreground font-mono text-xs">{value}</code>
      </span>
    );
  }
  if (value === null || value === undefined) {
    return <span className="text-faint-foreground">none</span>;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  return <code className="font-mono text-xs">{JSON.stringify(value)}</code>;
}
