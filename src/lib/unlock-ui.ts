const OPEN_OVERLAY =
  '[role="dialog"][data-state="open"], [role="alertdialog"][data-state="open"], [data-slot="sheet-content"][data-state="open"], [data-slot="dialog-content"][data-state="open"]';

/**
 * Modal parts (dialog, sheet, menu) lock the page while open: body gets pointer-events none,
 * a scroll lock, and siblings go inert. If the screen crashes while one is open, nothing
 * unlocks it and the whole window stops responding to the mouse. This puts the page back.
 */
export function unlockPage() {
  for (const el of [document.body, document.documentElement]) {
    el.style.removeProperty("pointer-events");
    el.style.removeProperty("overflow");
    el.style.removeProperty("padding-right");
    el.style.removeProperty("margin-right");
    el.removeAttribute("data-scroll-locked");
  }
  document.querySelectorAll("[inert]").forEach((el) => el.removeAttribute("inert"));
  document.querySelectorAll('body > [aria-hidden="true"]').forEach((el) => el.removeAttribute("aria-hidden"));
}

/** True when the page is locked but no overlay is actually open, which is the stuck state. */
export function isStuck() {
  return document.body.style.pointerEvents === "none" && !document.querySelector(OPEN_OVERLAY);
}

/** Heals a stuck page on the next click, so a crash behind a dialog never leaves the app dead. */
export function installUiUnlock() {
  const check = () => {
    if (isStuck()) unlockPage();
  };
  document.addEventListener("pointerdown", check, true);
  document.addEventListener("keydown", check, true);
  return () => {
    document.removeEventListener("pointerdown", check, true);
    document.removeEventListener("keydown", check, true);
  };
}
