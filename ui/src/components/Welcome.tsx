import type { Command, Example, Tool } from "@/api/model";
import { effectOf, railGroups } from "@/lib/rail";
import { EffectBadge } from "./EffectBadge";
import { CommandPathText } from "./Rail";

type Props = { tool: Tool; onExample: (command: Command, example: Example) => void };

/**
 * The first card of the transcript, built from the tool's own description:
 * its about line, then each heading's commands with their about text,
 * effect and examples. An example chip runs the example in a new card.
 * It is the tool's manual, so it sits on the page itself, with no box; the
 * runs after it are the cards.
 */
export function Welcome({ tool, onExample }: Props) {
  return (
    <section aria-label={`Welcome to ${tool.name}`} className="mb-3 px-4 py-3.5">
      <h1 className="m-0 font-mono text-base font-semibold">{tool.name}</h1>
      {tool.about !== null && <p className="text-muted-foreground mt-0.5 mb-2">{tool.about}</p>}
      {railGroups(tool).map((group) => (
        <div key={group.title ?? "commands"} className="mt-4">
          {group.title !== null && (
            <h2 className="text-faint-foreground m-0 mb-1.5 text-xs font-semibold">
              {group.title}
            </h2>
          )}
          <ul className="m-0 flex flex-col gap-2.5 p-0">
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
                  <div className="-mb-1 flex flex-wrap gap-x-3">
                    {command.examples.map((example) => (
                      <button
                        key={example.command}
                        type="button"
                        title={example.command}
                        onClick={() => onExample(command, example)}
                        className="text-primary active:text-foreground -mx-1 min-h-8 max-w-full truncate px-1 text-xs not-first:border-l not-first:pl-3 hover:underline"
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
