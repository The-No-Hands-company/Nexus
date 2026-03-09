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
const LS_FONT_FAMILY = "nexus:appearance:fontFamily";
const LS_REDUCED_MOTION = "nexus:appearance:reducedMotion";
const LS_RTL = "nexus:appearance:rtl";
const LS_COLOR_BLIND = "nexus:appearance:colorBlindMode";

export type FontSize = "sm" | "md" | "lg" | "xl";
export type Density = "compact" | "cozy" | "roomy";
export type ColorScheme = "system" | "dark" | "light";
export type FontFamily = "system" | "monospace" | "dyslexic" | "serif";
export type ColorBlindMode = "none" | "protanopia" | "deuteranopia" | "tritanopia";

const FONT_SIZES: FontSize[] = ["sm", "md", "lg", "xl"];
const DENSITIES: Density[] = ["compact", "cozy", "roomy"];
const FONT_FAMILIES: FontFamily[] = ["system", "monospace", "dyslexic", "serif"];
const COLOR_BLIND_MODES: ColorBlindMode[] = ["none", "protanopia", "deuteranopia", "tritanopia"];

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

export function applyFontFamily(family: FontFamily): void {
  const root = document.documentElement;
  FONT_FAMILIES.forEach((f) => root.classList.remove(`nexus-font-${f}`));
  if (family !== "system") root.classList.add(`nexus-font-${family}`);
}

export function applyReducedMotion(enabled: boolean): void {
  document.documentElement.classList.toggle("nexus-reduced-motion", enabled);
}

export function applyRtl(enabled: boolean): void {
  document.documentElement.classList.toggle("nexus-rtl", enabled);
}

export function applyColorBlindMode(mode: ColorBlindMode): void {
  const root = document.documentElement;
  COLOR_BLIND_MODES.forEach((m) => root.classList.remove(`nexus-cb-${m}`));
  if (mode !== "none") root.classList.add(`nexus-cb-${mode}`);
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

export function saveFontFamily(family: FontFamily): void {
  localStorage.setItem(LS_FONT_FAMILY, family);
  applyFontFamily(family);
}

export function saveReducedMotion(enabled: boolean): void {
  localStorage.setItem(LS_REDUCED_MOTION, String(enabled));
  applyReducedMotion(enabled);
}

export function saveRtl(enabled: boolean): void {
  localStorage.setItem(LS_RTL, String(enabled));
  applyRtl(enabled);
}

export function saveColorBlindMode(mode: ColorBlindMode): void {
  localStorage.setItem(LS_COLOR_BLIND, mode);
  applyColorBlindMode(mode);
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

export function getFontFamily(): FontFamily {
  return (localStorage.getItem(LS_FONT_FAMILY) as FontFamily | null) ?? "system";
}

export function getReducedMotion(): boolean {
  return localStorage.getItem(LS_REDUCED_MOTION) === "true";
}

export function getRtl(): boolean {
  return localStorage.getItem(LS_RTL) === "true";
}

export function getColorBlindMode(): ColorBlindMode {
  return (localStorage.getItem(LS_COLOR_BLIND) as ColorBlindMode | null) ?? "none";
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
  applyFontFamily(getFontFamily());
  applyReducedMotion(getReducedMotion());
  applyRtl(getRtl());
  applyColorBlindMode(getColorBlindMode());
}
