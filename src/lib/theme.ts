import { getCurrentWindow } from "@tauri-apps/api/window";
import { flushSync } from "svelte";
import { inTauri } from "#lib/env.js";
import {
  ensureRevealStyle,
  runThemeReveal,
  supportsViewTransition,
} from "#lib/components/ui/theme-toggle/index.js";

export type ThemePreference = "system" | "light" | "dark";

/** Applies a theme now and remembers it for the next launch (static/theme.js reads the same key). */
/** Makes the native titlebar follow the app theme; null hands control back to the system. */
export function syncWindowTheme(pref: ThemePreference) {
  if (!inTauri()) return;
  void getCurrentWindow()
    .setTheme(pref === "system" ? null : pref)
    .catch(() => {
      // A window without the theme permission keeps its system titlebar.
    });
}

export function applyTheme(pref: ThemePreference) {
  syncWindowTheme(pref);
  localStorage.setItem("airlock-theme", pref);
  const dark = pref === "dark" || (pref === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", dark);
}

/**
 * Runs a theme change inside Baby UI's page reveal (View Transition). Falls back to an
 * instant change under reduced motion or when the browser has no View Transition support.
 */
export function withThemeReveal(apply: () => void) {
  ensureRevealStyle();
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (reduced || !supportsViewTransition()) {
    apply();
    return;
  }
  // The new snapshot is taken when the callback returns, so the DOM must be updated by then.
  runThemeReveal("rectangle", "bottom-up", () => flushSync(apply));
}
