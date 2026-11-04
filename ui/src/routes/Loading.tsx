import { Topbar } from "@/components/Topbar";

/** A grey bar of the skeleton, `width` wide, that pulses while the page loads. */
function Bar({ width }: { width: string }) {
  return <div className={`bg-muted h-3.5 rounded motion-safe:animate-pulse ${width}`} />;
}

/**
 * The page while it signs in and loads the model: the topbar, a skeleton
 * rail and a skeleton welcome card, each in the place its content takes.
 */
export function Loading() {
  return (
    <div className="flex h-full flex-col">
      <Topbar name="" version="" readOnly={false} onFind={null} />
      <div className="flex min-h-0 flex-1">
        <nav
          aria-busy="true"
          aria-label="Commands"
          className="bg-rail hidden w-64 shrink-0 flex-col gap-2.5 border-r p-3 md:flex"
        >
          {["w-2/5", "w-3/4", "w-2/3", "w-3/4", "w-2/5", "w-2/3", "w-3/4"].map((width, index) => (
            // The bars never reorder.
            // biome-ignore lint/suspicious/noArrayIndexKey: the skeleton's bars are fixed
            <Bar key={index} width={width} />
          ))}
        </nav>
        <main aria-busy="true" className="min-w-0 flex-1 px-4 pt-3">
          <div className="bg-card flex flex-col gap-3 rounded-lg border px-4 py-4 shadow-xs">
            <Bar width="w-32" />
            <Bar width="w-2/3" />
            <Bar width="w-1/2" />
            <Bar width="w-3/5" />
          </div>
        </main>
      </div>
    </div>
  );
}
