import { useQuery } from "@tanstack/react-query";
import { lazy, Suspense, useEffect, useState } from "react";
import { dropToken } from "@/api/client";
import { deckQuery } from "@/api/deck";
import { isSignedOut } from "@/api/failure";
import { modelQuery } from "@/api/model";
import { sessionQuery } from "@/api/session";
import { Failure } from "@/components/Failure";
import { usePlace } from "@/lib/place";
import { Loading } from "@/routes/Loading";
import { SignedOut } from "@/routes/SignedOut";
import { ToolPage } from "@/routes/ToolPage";

// The workspaces pages load in their own file the first time they show, so
// the page's first load stays within its size budget.
const Deck = lazy(() => import("@/routes/workspaces/Deck"));

/**
 * Signs the page in with the launch code from its address, then asks the
 * server whether it holds a tool. Without one the page shows the
 * workspaces. With one it loads the tool's model and shows the tool's page
 * at the start, and the workspaces at their own addresses, keeping the
 * tool's page and its runs while they show. A page without a session, or
 * whose token the server refuses, says how to start again.
 */
export function App({ code }: { code: string | null }) {
  const session = useQuery(sessionQuery(code));
  const signedIn = session.data === "signed-in";
  const deck = useQuery({ ...deckQuery(), enabled: signedIn });
  const tool = deck.data?.tool === true;
  const model = useQuery({ ...modelQuery(), enabled: signedIn && tool });
  const [refused, setRefused] = useState(false);
  const signedOut = refused || isSignedOut(deck.error) || isSignedOut(model.error);
  const place = usePlace();
  const signOut = () => setRefused(true);

  // A refused token is forgotten, so a reload does not send it again.
  useEffect(() => {
    if (signedOut) {
      dropToken();
    }
  }, [signedOut]);

  if (session.isPending) {
    return <Loading />;
  }
  if (session.isError) {
    return <Failure error={session.error} onRetry={() => void session.refetch()} />;
  }
  if (!signedIn || signedOut) {
    return <SignedOut />;
  }
  if (deck.isPending) {
    return <Loading />;
  }
  if (deck.isError) {
    return <Failure error={deck.error} onRetry={() => void deck.refetch()} />;
  }
  if (tool && model.isPending) {
    return <Loading />;
  }
  if (tool && model.isError) {
    return <Failure error={model.error} onRetry={() => void model.refetch()} />;
  }
  const away = !tool || place.view !== "start";
  return (
    <>
      {away && (
        <Suspense fallback={<Loading />}>
          <Deck place={place} tool={tool} onSignedOut={signOut} />
        </Suspense>
      )}
      {model.data !== undefined && (
        <div hidden={away} className="h-full">
          <ToolPage reply={model.data} onSignedOut={signOut} />
        </div>
      )}
    </>
  );
}
