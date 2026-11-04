import { cn } from "cn";
import { Lock } from "lucide-react";
import type { Command, Tool } from "@/api/model";
import { effectOf, railGroups, whyNotRunnable } from "@/lib/rail";
import { EffectBadge } from "./EffectBadge";

type Props = {
  tool: Tool;
  selected: Command | null;
  readOnly: boolean;
  onChoose: (command: Command) => void;
};

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
 * in help order, each command with its effect's dot and word. The topbar
 * names the tool, so the rail does not. Under `--read-only` a command that
 * is not read-only stays in the rail, dimmed and with a lock, and choosing
 * it shows its form with the reason. A window narrower than the `md`
 * breakpoint hides the rail, and the welcome card and ⌘K list the
 * commands instead.
 */
export function Rail({ tool, selected, readOnly, onChoose }: Props) {
  return (
    <nav
      aria-label="Commands"
      className="bg-rail hidden w-64 shrink-0 overflow-y-auto border-r px-2 py-3 md:block"
    >
      {railGroups(tool).map((group, index) => (
        <div key={group.title ?? "commands"} className={cn(index > 0 && "mt-4")}>
          {group.title !== null && (
            <h2 className="text-faint-foreground mx-2 mb-1 text-xs font-semibold">{group.title}</h2>
          )}
          <ul className="m-0 p-0">
            {group.commands.map((command) => {
              const locked = whyNotRunnable(command, readOnly) !== null;
              return (
                <li key={command.path.join(" ")} className="list-none">
                  <button
                    type="button"
                    aria-current={selected === command ? "true" : undefined}
                    onClick={() => onChoose(command)}
                    className={cn(
                      "transition-tint active:bg-muted flex min-h-8 w-full items-center gap-2 rounded-md py-1 pr-2 pl-2 text-left",
                      selected === command
                        ? "bg-card lift-card font-semibold"
                        : "hover:bg-card/60 dark:hover:bg-muted",
                      locked && "text-faint-foreground",
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate">
                      <CommandPathText command={command} />
                    </span>
                    {locked && (
                      <>
                        <Lock aria-hidden="true" className="size-3" />
                        <span className="sr-only">turned off by --read-only</span>
                      </>
                    )}
                    <EffectBadge effect={effectOf(command)} quiet />
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}
