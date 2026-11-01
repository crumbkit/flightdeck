import { Topbar } from "@/components/Topbar";

/** The page while it signs in and loads the model: the topbar and a skeleton rail. */
export function Loading() {
  return (
    <div className="flex h-full flex-col">
      <Topbar name="" version="" readOnly={false} onFind={null} />
      <div className="flex min-h-0 flex-1">
        <nav aria-busy="true" aria-label="Commands" className="bg-rail w-64 shrink-0 border-r p-3">
          {[0, 1, 2, 3, 4, 5].map((bar) => (
            <div
              key={bar}
              className="bg-muted mb-2.5 h-3.5 w-3/4 rounded motion-safe:animate-pulse"
            />
          ))}
        </nav>
      </div>
    </div>
  );
}
