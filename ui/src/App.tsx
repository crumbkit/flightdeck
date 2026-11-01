import { useQuery } from "@tanstack/react-query";
import { dropToken } from "@/api/client";
import { isSignedOut } from "@/api/failure";
import { modelQuery } from "@/api/model";
import { sessionQuery } from "@/api/session";
import { Failure } from "@/components/Failure";
import { Loading } from "@/routes/Loading";
import { SignedOut } from "@/routes/SignedOut";
import { ToolPage } from "@/routes/ToolPage";

/**
 * Signs the page in with the launch code from its address, then loads the
 * tool's model and shows its page. A page without a session, or whose
 * token the server refuses, says how to start again.
 */
export function App({ code }: { code: string | null }) {
  const session = useQuery(sessionQuery(code));
  const signedIn = session.data === "signed-in";
  const model = useQuery({ ...modelQuery(), enabled: signedIn });

  if (session.isPending) {
    return <Loading />;
  }
  if (session.isError) {
    return <Failure error={session.error} onRetry={() => void session.refetch()} />;
  }
  if (!signedIn || isSignedOut(model.error)) {
    if (isSignedOut(model.error)) {
      dropToken();
    }
    return <SignedOut />;
  }
  if (model.isPending) {
    return <Loading />;
  }
  if (model.isError) {
    return <Failure error={model.error} onRetry={() => void model.refetch()} />;
  }
  return <ToolPage reply={model.data} />;
}
