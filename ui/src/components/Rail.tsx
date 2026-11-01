import { cn } from "cn";
import type { Command, Tool } from "@/api/model";
import { effectOf, railGroups } from "@/lib/rail";
import { EffectBadge } from "./EffectBadge";

type Props = { tool: Tool; selected: Command | null; onChoose: (command: Command) => void };

/** A command's path with its resource greyed, such as books list. */
export function CommandPathText({ command }: { command: Command }) {
  const verb = command.path.at(-1);
  const groups = command.path.slice(0, -1).join(" ");
  return (
    <span className="font-mono">
      {groups !== "" && <span className="text-faint-foreground">{groups}</span>}
      {groups !== "" && " "}
      {verb}
    </span>
  );
}

/**
 * The rail of commands, grouped under the goal headings of the tool's help
 * in help order, each command with its effect badge and its word.
 */
export function Rail({ tool, selected, onChoose }: Props) {
  return (
    <nav aria-label="Commands" className="bg-rail w-64 shrink-0 overflow-y-auto border-r px-2 py-2">
      <p className="px-2 pb-1 font-semibold">
        {tool.name}{" "}
        <span className="text-faint-foreground font-mono text-xs font-normal">{tool.version}</span>
      </p>
      {railGroups(tool).map((group) => (
        <div key={group.title ?? "commands"} className="mt-2">
          {group.title !== null && (
            <h2 className="text-faint-foreground mx-2 mb-0.5 text-xs font-semibold tracking-wide uppercase">
              {group.title}
            </h2>
          )}
          <ul className="m-0 p-0">
            {group.commands.map((command) => (
              <li key={command.path.join(" ")} className="list-none">
                <button
                  type="button"
                  aria-current={selected === command ? "true" : undefined}
                  onClick={() => onChoose(command)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-md py-1 pr-1 pl-3 text-left",
                    selected === command && "bg-card font-semibold shadow-xs",
                  )}
                >
                  <span className="min-w-0 flex-1 truncate">
                    <CommandPathText command={command} />
                  </span>
                  <EffectBadge effect={effectOf(command)} />
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </nav>
  );
}
