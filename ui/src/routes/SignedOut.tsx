import { CopyButton } from "@/components/CopyButton";

const restart = "flightdeck open <TOOL>";

/**
 * The page without a session: its launch code expired or was used, or the
 * server refused its token. Without the token the page can run nothing, so
 * it says how to start a new page.
 */
export function SignedOut() {
  return (
    <main className="mx-auto mt-16 max-w-lg px-4">
      <section aria-labelledby="signed-out" className="bg-card rounded-lg border p-5">
        <h1 id="signed-out" className="mb-2 text-base font-semibold">
          This page is not signed in
        </h1>
        <p className="text-muted-foreground mb-3">
          Its sign-in has expired or was already used. Run this in your terminal, with the tool’s
          name, to start again:
        </p>
        <div className="bg-code flex items-center gap-3 rounded-md border px-3 py-2 font-mono">
          <code className="flex-1">{restart}</code>
          <CopyButton text={restart} label="Copy" />
        </div>
      </section>
    </main>
  );
}
