import type { Command, Example, Tool } from "@/api/model";
import { effectOf, railGroups } from "@/lib/rail";
import { EffectBadge } from "./EffectBadge";
import { CommandPathText } from "./Rail";

type Props = { tool: Tool; onExample: (command: Command, example: Example) => void };

/**
 * The first card of the transcript, built from the tool's own description:
 * its about line, then each heading's commands with their about text,
 * effect and examples. An example chip runs the example in a new card.
 */
export function Welcome({ tool, onExample }: Props) {
  return (
    <section
      aria-label={`Welcome to ${tool.name}`}
      className="mb-3 rounded-lg border border-dashed px-4 py-3"
    >
      <h1 className="m-0 text-base font-semibold">
        <span className="font-mono">{tool.name}</span>{" "}
        <span className="text-faint-foreground font-mono text-xs font-normal">{tool.version}</span>
      </h1>
      {tool.about !== null && <p className="text-muted-foreground mt-0.5 mb-2">{tool.about}</p>}
      {railGroups(tool).map((group) => (
        <div key={group.title ?? "commands"} className="mt-3">
          {group.title !== null && (
            <h2 className="text-faint-foreground m-0 mb-1 text-xs font-semibold tracking-wide uppercase">
              {group.title}
            </h2>
          )}
          <ul className="m-0 flex flex-col gap-2 p-0">
            {group.commands.map((command) => (
              <li key={command.path.join(" ")} className="list-none">
                <div className="flex flex-wrap items-center gap-2">
                  <CommandPathText command={command} />
                  <EffectBadge effect={effectOf(command)} />
                  {command.about !== null && (
                    <span className="text-muted-foreground">{command.about}</span>
                  )}
                </div>
                {command.examples.length > 0 && (
                  <div className="mt-1 flex flex-wrap gap-1.5">
                    {command.examples.map((example) => (
                      <button
                        key={example.command}
                        type="button"
                        title={example.command}
                        onClick={() => onExample(command, example)}
                        className="bg-muted rounded-full border px-2.5 py-0.5 text-xs"
                      >
                        {example.about}
                      </button>
                    ))}
                  </div>
                )}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </section>
  );
}
