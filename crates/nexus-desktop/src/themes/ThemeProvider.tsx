import { useEffect } from "react";
import { useStore } from "../store";
import { getTheme } from "./themes";

/** Applies the active theme's CSS vars to <html> whenever the theme changes. */
export default function ThemeProvider({ children }: { children: React.ReactNode }) {
  const activeThemeId = useStore((s) => s.activeThemeId);

  useEffect(() => {
    const theme = getTheme(activeThemeId);
    const root = document.documentElement;
    for (const [key, value] of Object.entries(theme.vars)) {
      root.style.setProperty(key, value);
    }
  }, [activeThemeId]);

  return <>{children}</>;
}
