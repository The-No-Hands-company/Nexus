/**
 * Appearance preferences — font size, message density, and color scheme.
 *
 * All preferences are persisted to localStorage and applied to the
 * `document.documentElement` via CSS classes.  Call `initAppearance()` once
 * at application startup to restore saved settings.
 */

const LS_FONT = "nexus:appearance:fontSize";
const LS_DENSITY = "nexus:appearance:density";
const LS_SCHEME = "nexus:appearance:colorScheme";

export type FontSize = "sm" | "md" | "lg" | "xl";
export type Density = "compact" | "cozy" | "roomy";
export type ColorScheme = "system" | "dark" | "light";

const FONT_SIZES: FontSize[] = ["sm", "md", "lg", "xl"];
const DENSITIES: Density[] = ["compact", "cozy", "roomy"];

// ─── DOM helpers ──────────────────────────────────────────────────────────────

export function applyFontSize(size: FontSize): void {
  const root = document.documentElement;
  FONT_SIZES.forEach((s) => root.classList.remove(`nexus-font-${s}`));
  root.classList.add(`nexus-font-${size}`);
}

export function applyDensity(density: Density): void {
  const root = document.documentElement;
  DENSITIES.forEach((d) => root.classList.remove(`nexus-density-${d}`));
  root.classList.add(`nexus-density-${density}`);
}

// ─── Persistence helpers ──────────────────────────────────────────────────────

export function saveFontSize(size: FontSize): void {
  localStorage.setItem(LS_FONT, size);
  applyFontSize(size);
}

export function saveDensity(density: Density): void {
  localStorage.setItem(LS_DENSITY, density);
  applyDensity(density);
}

export function saveColorScheme(scheme: ColorScheme): void {
  localStorage.setItem(LS_SCHEME, scheme);
}

// ─── Getters (with defaults) ──────────────────────────────────────────────────

export function getFontSize(): FontSize {
  return (localStorage.getItem(LS_FONT) as FontSize | null) ?? "md";
}

export function getDensity(): Density {
  return (localStorage.getItem(LS_DENSITY) as Density | null) ?? "cozy";
}

export function getColorScheme(): ColorScheme {
  return (localStorage.getItem(LS_SCHEME) as ColorScheme | null) ?? "system";
}

/**
 * Derive whether the effective color scheme is dark.
 * "system" respects the OS preference.
 */
export function isDarkScheme(scheme: ColorScheme): boolean {
  if (scheme === "dark") return true;
  if (scheme === "light") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

// ─── Startup ──────────────────────────────────────────────────────────────────

/**
 * Restore saved appearance settings from localStorage.
 * Call this once, before React mounts.
 */
export function initAppearance(): void {
  applyFontSize(getFontSize());
  applyDensity(getDensity());
}
