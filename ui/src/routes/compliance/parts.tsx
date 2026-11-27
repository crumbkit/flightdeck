// What the three Compliance site pages share: a state's chip, a sentence
// with its paths and commits in the mono face, the link to a repository's
// page and the link to the CI run that made a report.
import type { RepositoryColumn } from "@crumbkit/flightdeck-headless/document";
import type { Phrase } from "@crumbkit/flightdeck-headless/words";
import type { ReactNode } from "react";
import { placeHash } from "@/compliance-site/address";

/** A sentence, with its paths, commits and measures in the mono face. */
export function PhraseText({ phrase }: { readonly phrase: Phrase }) {
  return (
    <>
      {phrase.map((piece, at) =>
        typeof piece === "string" ? (
          piece
        ) : (
          // biome-ignore lint/suspicious/noArrayIndexKey: a phrase's pieces never move, so their place is their key.
          <span key={at} className="cs-mono">
            {piece.mono}
          </span>
        ),
      )}
    </>
  );
}

/** Returns the class of a state's chip, by the word it shows: the state's colour, or a quiet grey for any other word. */
export function chipClass(word: string): string {
  switch (word) {
    case "failing":
      return "cs-chip cs-state cs-failing";
    case "stale":
      return "cs-chip cs-state cs-stale";
    case "current":
    case "matches":
      return "cs-chip cs-state cs-current";
    default:
      return "cs-chip cs-state cs-quiet";
  }
}

/** A state's chip: its word, in its colour. */
export function StateChip({ word }: { readonly word: string }) {
  return <span className={chipClass(word)}>{word}</span>;
}

/** Returns the hash of a repository's page, with `rule` selected when given. */
export function repositoryHash(name: string, rule: string | null = null): string {
  const search = rule === null ? "" : `?rule=${encodeURIComponent(rule)}`;
  return placeHash({ page: "repository", name }, search);
}

/** A link to a repository's page, in the mono face. */
export function RepositoryLink({
  name,
  rule = null,
}: {
  readonly name: string;
  readonly rule?: string | null;
}) {
  return (
    <a className="cs-mono" href={repositoryHash(name, rule)}>
      {name}
    </a>
  );
}

/** Returns the address of the GitHub Actions run that made a column's report, or null for a report from no CI run. */
export function runUrl(column: RepositoryColumn | undefined): string | null {
  const run = column?.source_run ?? null;
  if (run === null) return null;
  return `https://github.com/${run.repository}/actions/runs/${run.run_id}`;
}

/**
 * A link that leaves the page, such as to a CI run. It opens in a new tab,
 * which a host that frames the page without popups refuses, so the page
 * itself never navigates away.
 */
export function OutLink({
  href,
  className,
  children,
}: {
  readonly href: string;
  readonly className?: string;
  readonly children: ReactNode;
}) {
  return (
    <a
      className={className === undefined ? "cs-ext" : `${className} cs-ext`}
      href={href}
      target="_blank"
      rel="noreferrer"
    >
      {children}
    </a>
  );
}

/** The run that made a column's report, as a link when the report came from a CI run. */
export function RunLink({
  column,
  label,
  className,
}: {
  readonly column: RepositoryColumn | undefined;
  readonly label: string;
  readonly className?: string;
}) {
  const href = runUrl(column);
  if (href === null) return <span className={className}>{label}</span>;
  return (
    <OutLink href={href} {...(className === undefined ? {} : { className })}>
      {label}
    </OutLink>
  );
}
