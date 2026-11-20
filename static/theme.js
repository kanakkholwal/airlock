// Applies the saved or system colour scheme before first paint. Loaded as a file: the CSP blocks inline scripts.
const query = matchMedia("(prefers-color-scheme: dark)");
const apply = () => {
  const pref = localStorage.getItem("airlock-theme") ?? "system";
  document.documentElement.classList.toggle("dark", pref === "dark" || (pref === "system" && query.matches));
};
apply();
query.addEventListener("change", apply);
