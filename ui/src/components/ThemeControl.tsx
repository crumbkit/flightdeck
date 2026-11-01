import { useState } from "react";
import { readTheme, type Theme, writeTheme } from "@/storage";

/** Writes the theme choice onto the root element, where the tokens read it. */
export function applyTheme(theme: Theme): void {
  if (theme === "system") {
    delete document.documentElement.dataset.theme;
  } else {
    document.documentElement.dataset.theme = theme;
  }
}

const choices: { theme: Theme; label: string }[] = [
  { theme: "system", label: "System" },
  { theme: "light", label: "Light" },
  { theme: "dark", label: "Dark" },
];

/** The System, Light and Dark control in the topbar. */
export function ThemeControl() {
  const [theme, setTheme] = useState(readTheme);
  const choose = (next: Theme) => {
    setTheme(next);
    writeTheme(next);
    applyTheme(next);
  };
  return (
    <fieldset className="flex gap-0.5 rounded-md border p-0.5 text-xs">
      <legend className="sr-only">Appearance</legend>
      {choices.map((choice) => (
        <label
          key={choice.theme}
          className="text-muted-foreground has-checked:bg-muted has-checked:text-foreground has-focus-visible:outline-ring cursor-pointer rounded px-2 py-0.5 has-checked:font-semibold has-focus-visible:outline-2"
        >
          <input
            type="radio"
            name="theme"
            className="sr-only"
            checked={theme === choice.theme}
            onChange={() => choose(choice.theme)}
          />
          {choice.label}
        </label>
      ))}
    </fieldset>
  );
}
