import { writable } from "svelte/store";
import type { ThemeName } from "../types";

const STORAGE_KEY = "atiga-cine-theme";
const THEME_STYLES: Partial<Record<ThemeName, () => Promise<unknown>>> = {
  "charcoal-amber": () => import("../styles/skins/charcoal-amber.css"),
  "silver-classic": () => import("../styles/skins/silver-classic.css"),
  "winamp-classic": () => import("../styles/skins/winamp-classic.css"),
};
export const AVAILABLE_THEMES = [
  { name: "retro-tv", label: "RETRO TV" },
] as const satisfies readonly { name: ThemeName; label: string }[];

const fallbackTheme: ThemeName = AVAILABLE_THEMES[0].name;

function isThemeName(value: string): value is ThemeName {
  return AVAILABLE_THEMES.some((theme) => theme.name === value);
}

function readTheme(): ThemeName {
  if (typeof localStorage === "undefined") return fallbackTheme;

  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved && isThemeName(saved) ? saved : fallbackTheme;
  } catch {
    return fallbackTheme;
  }
}

function applyTheme(value: ThemeName): void {
  if (typeof document === "undefined") return;
  document.documentElement.setAttribute("data-theme", value);
}

function loadThemeStyles(value: ThemeName): void {
  const load = THEME_STYLES[value];
  if (load) void load().catch(() => undefined);
}

const initialTheme = readTheme();
loadThemeStyles(initialTheme);
applyTheme(initialTheme);

export const themeStore = writable<ThemeName>(initialTheme);

export function setTheme(value: ThemeName): void {
  loadThemeStyles(value);
  applyTheme(value);
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // The visual theme still applies when storage is unavailable.
  }
  themeStore.set(value);
}
