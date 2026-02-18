export interface ThemeDefinition {
  id: string;
  name: string;
  vars: Record<string, string>;
}

const nexusDark: ThemeDefinition = {
  id: "nexus-dark",
  name: "Nexus Dark",
  vars: {
    "--color-bg-900": "#0d0f13",
    "--color-bg-800": "#161a22",
    "--color-bg-700": "#1e2330",
    "--color-bg-600": "#262d3d",
    "--color-bg-500": "#2f3850",
    "--color-accent-500": "#7c6af7",
    "--color-accent-600": "#6b59e8",
    "--color-accent-400": "#9d8fff",
    "--color-surface-900": "#111318",
    "--color-surface-800": "#191d26",
    "--color-surface-700": "#222736",
    "--color-muted": "#8892a4",
    "--color-online": "#3ba55c",
    "--color-idle": "#faa81a",
    "--color-dnd": "#ed4245",
    "--color-offline": "#747f8d",
    "--color-fg": "#e2e8f0",
    "--scrollbar-thumb": "#2f3850",
    "--scrollbar-thumb-hover": "#3d4a63",
  },
};

const midnight: ThemeDefinition = {
  id: "midnight",
  name: "Midnight",
  vars: {
    "--color-bg-900": "#060709",
    "--color-bg-800": "#0e1016",
    "--color-bg-700": "#141820",
    "--color-bg-600": "#1b2030",
    "--color-bg-500": "#222840",
    "--color-accent-500": "#5865f2",
    "--color-accent-600": "#4752c4",
    "--color-accent-400": "#7984f5",
    "--color-surface-900": "#08090c",
    "--color-surface-800": "#101318",
    "--color-surface-700": "#181c26",
    "--color-muted": "#72809a",
    "--color-online": "#3ba55c",
    "--color-idle": "#faa81a",
    "--color-dnd": "#ed4245",
    "--color-offline": "#747f8d",
    "--color-fg": "#dce2f0",
    "--scrollbar-thumb": "#222840",
    "--scrollbar-thumb-hover": "#2e3856",
  },
};

const ocean: ThemeDefinition = {
  id: "ocean",
  name: "Ocean",
  vars: {
    "--color-bg-900": "#071520",
    "--color-bg-800": "#0d1f30",
    "--color-bg-700": "#132840",
    "--color-bg-600": "#1a3254",
    "--color-bg-500": "#223d66",
    "--color-accent-500": "#00b4d8",
    "--color-accent-600": "#0096c7",
    "--color-accent-400": "#48cae4",
    "--color-surface-900": "#081720",
    "--color-surface-800": "#0f2030",
    "--color-surface-700": "#162a40",
    "--color-muted": "#6e8fa8",
    "--color-online": "#2dc653",
    "--color-idle": "#f9a825",
    "--color-dnd": "#e53935",
    "--color-offline": "#607d8b",
    "--color-fg": "#d4eaf5",
    "--scrollbar-thumb": "#223d66",
    "--scrollbar-thumb-hover": "#2e507f",
  },
};

const nexusLight: ThemeDefinition = {
  id: "nexus-light",
  name: "Nexus Light",
  vars: {
    "--color-bg-900": "#f0f2f5",
    "--color-bg-800": "#e3e6ea",
    "--color-bg-700": "#d6dae0",
    "--color-bg-600": "#c9cdd5",
    "--color-bg-500": "#b5bbc6",
    "--color-accent-500": "#7c6af7",
    "--color-accent-600": "#6b59e8",
    "--color-accent-400": "#9d8fff",
    "--color-surface-900": "#e8eaee",
    "--color-surface-800": "#dde0e6",
    "--color-surface-700": "#d2d6de",
    "--color-muted": "#5c6478",
    "--color-online": "#3ba55c",
    "--color-idle": "#e0901a",
    "--color-dnd": "#d83737",
    "--color-offline": "#607d8b",
    "--color-fg": "#1a1d26",
    "--scrollbar-thumb": "#b5bbc6",
    "--scrollbar-thumb-hover": "#9aa0ad",
  },
};

export const BUILTIN_THEMES: ThemeDefinition[] = [
  nexusDark,
  midnight,
  ocean,
  nexusLight,
];

export const DEFAULT_THEME_ID = nexusDark.id;

export function getTheme(id: string): ThemeDefinition {
  return BUILTIN_THEMES.find((t) => t.id === id) ?? nexusDark;
}
