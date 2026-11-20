export type ThemePreference = "system" | "light" | "dark";

/** Applies a theme now and remembers it for the next launch (static/theme.js reads the same key). */
export function applyTheme(pref: ThemePreference) {
  localStorage.setItem("airlock-theme", pref);
  const dark = pref === "dark" || (pref === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.classList.toggle("dark", dark);
}
