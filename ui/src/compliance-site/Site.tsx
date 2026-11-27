// The Compliance site: the top bar with its two tabs, the snapshot note, and
// the page the hash names, all drawn from the document inlined in the file.
// A document that is missing or fails the schema draws one line with its
// code and nothing else.
import { complianceAddress, complianceView } from "@crumbkit/flightdeck-headless/compliance";
import type { ComplianceSource } from "@crumbkit/flightdeck-headless/source";
import { useComplianceDocument } from "@crumbkit/flightdeck-react/useComplianceDocument";
import { useMemo } from "react";
import { CompliancePage } from "@/routes/compliance/CompliancePage";
import { OutLink } from "@/routes/compliance/parts";
import { RepositoryPage } from "@/routes/compliance/RepositoryPage";
import { RulesPage } from "@/routes/compliance/RulesPage";
import { placeHash, usePlace } from "./address.ts";

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Returns an RFC 3339 time as the top bar shows it, such as "25 Nov 2026, 06:00 UTC". */
export function topBarTime(iso: string): string {
  const at = new Date(iso);
  const hours = String(at.getUTCHours()).padStart(2, "0");
  const minutes = String(at.getUTCMinutes()).padStart(2, "0");
  return `${at.getUTCDate()} ${MONTHS[at.getUTCMonth()] ?? ""} ${at.getUTCFullYear()}, ${hours}:${minutes} UTC`;
}

/** Returns the code of `error`, or null when it has none. */
function codeOf(error: unknown): string | null {
  if (!(error instanceof Error) || !("code" in error)) return null;
  return typeof error.code === "string" ? error.code : null;
}

/**
 * Returns the line a failed read draws: the code and the message of the
 * failure, or of its cause when the cause has a code of its own, such as a
 * page with no document element.
 */
export function failureLine(error: Error): string {
  const cause = error.cause;
  const inner = cause instanceof Error && codeOf(cause) !== null ? cause : error;
  return `flightdeck::compliance::${codeOf(inner) ?? "invalid_document"}: ${inner.message}`;
}

/** The build run the hero names: a link to its run when the build knows the address, or its label. */
function BuildRun({ label, url }: { readonly label: string | null; readonly url: string | null }) {
  if (url === null) return <span>{label}</span>;
  return <OutLink href={url}>{label ?? "Build run"}</OutLink>;
}

/** The site, reading its document from `source`. */
export function Site({ source }: { readonly source: ComplianceSource }) {
  const { document, error } = useComplianceDocument(source);
  const { place, navigate } = usePlace();
  // The note above every page, which the headless views share.
  const snapshot = useMemo(
    () =>
      document === null ? null : complianceView(document, complianceAddress.parse("")).snapshot,
    [document],
  );
  if (error !== null) {
    return (
      <p className="cs-failure" role="alert">
        {failureLine(error)}
      </p>
    );
  }
  if (document === null || snapshot === null) return null;
  const route = place.route;
  const onRules = route.page === "rules";
  const run = snapshot.buildRun;
  return (
    <div className="cs-page">
      <header className="cs-top">
        <div className="cs-row cs-nowrap" style={{ gap: 18 }}>
          <span className="cs-brand">flightdeck</span>
          <nav className="cs-tabs" aria-label="Pages">
            <a href={placeHash({ page: "compliance" })} aria-current={onRules ? undefined : "page"}>
              Compliance
            </a>
            <a href={placeHash({ page: "rules" })} aria-current={onRules ? "page" : undefined}>
              Rules
            </a>
          </nav>
        </div>
        <div className="cs-row cs-nowrap">
          <span className="cs-cap">Snapshot of {topBarTime(document.built_at)}</span>
          <span className="cs-chip">read only</span>
        </div>
      </header>
      <main className="cs-main">
        <div className="cs-hero" role="note">
          <div>{snapshot.sentence}</div>
          <div className="cs-row cs-nowrap">
            {run === null ? null : <BuildRun label={run.label} url={run.url} />}
            <span className="cs-cap">{snapshot.from}</span>
          </div>
        </div>
        {route.page === "compliance" ? (
          <CompliancePage document={document} search={place.search} navigate={navigate} />
        ) : null}
        {route.page === "rules" ? (
          <RulesPage document={document} search={place.search} navigate={navigate} />
        ) : null}
        {route.page === "repository" ? (
          <RepositoryPage
            key={route.name}
            document={document}
            name={route.name}
            search={place.search}
            navigate={navigate}
          />
        ) : null}
      </main>
    </div>
  );
}
