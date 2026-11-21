import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
  requestAccessibilityPermission,
  requestMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { commands } from "../../bindings";

export const isMac = () => typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");

export type DictationAccess = { accessibility: boolean; microphone: boolean };

/** macOS asks for these two permissions; other systems report both as granted. */
export async function checkAccess(): Promise<DictationAccess> {
  if (!isMac()) return { accessibility: true, microphone: true };
  const [accessibility, microphone] = await Promise.all([
    checkAccessibilityPermission().catch(() => false),
    checkMicrophonePermission().catch(() => false),
  ]);
  return { accessibility, microphone };
}

export async function requestAccess(kind: keyof DictationAccess) {
  if (!isMac()) return;
  await (kind === "accessibility" ? requestAccessibilityPermission() : requestMicrophonePermission());
}

/**
 * Decision (architecture): the backend does not register the shortcut or the paste engine by
 * itself; the frontend must call these once permissions allow (see initialize_core_logic in
 * lib.rs). Both are idempotent, so calling again after a permission grant is safe.
 */
export async function startDictationEngine(): Promise<string | null> {
  const enigo = await commands.initializeEnigo();
  if (enigo.status === "error") return enigo.error;
  const shortcuts = await commands.initializeShortcuts();
  return shortcuts.status === "error" ? shortcuts.error : null;
}
