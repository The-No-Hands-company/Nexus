import { BUILTIN_THEMES } from "./themes";
import { useStore } from "../store";

/** A dropdown that lets users switch the active theme. */
export default function ThemeSwitcher() {
  const { activeThemeId, setActiveTheme } = useStore();

  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium text-muted uppercase tracking-wider">
        Theme
      </label>
      <select
        value={activeThemeId}
        onChange={(e) => setActiveTheme(e.target.value)}
        className="input text-sm"
      >
        {BUILTIN_THEMES.map((theme) => (
          <option key={theme.id} value={theme.id}>
            {theme.name}
          </option>
        ))}
      </select>
      <p className="text-xs text-muted mt-1">
        Theme changes apply immediately.
      </p>
    </div>
  );
}
