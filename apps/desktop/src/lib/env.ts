/** True inside the desktop app; false in a plain browser preview where there is no backend. */
export const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
