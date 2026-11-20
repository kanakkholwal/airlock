import { commands, type OverlayStyle, type ShortcutActivation } from "../bindings";

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Recorder tokens use "mod"; the Rust key parser wants concrete modifiers.
const toBinding = (v: string) => v.replace(/\bmod\b/g, navigator.userAgent.includes("Mac") ? "cmd" : "ctrl");

class DictationSettings {
  loaded = $state(false);
  error = $state<string | null>(null);
  shortcut = $state<string | null>("ctrl+space");
  activation = $state<ShortcutActivation>("hold_or_toggle");
  overlayStyle = $state<OverlayStyle>("minimal");
  cleanupWithLlm = $state(false);
  muteWhileRecording = $state(true);

  async load() {
    if (!inTauri()) return;
    const res = await commands.getAppSettings();
    if (res.status === "error") {
      this.error = res.error;
      return;
    }
    const s = res.data;
    this.shortcut = s.bindings?.transcribe?.current_binding ?? this.shortcut;
    this.activation = s.shortcut_activation ?? this.activation;
    this.overlayStyle = s.overlay_style ?? this.overlayStyle;
    this.cleanupWithLlm = s.post_process_enabled ?? false;
    this.muteWhileRecording = s.mute_while_recording ?? true;
    this.loaded = true;
  }

  private async apply<T>(call: () => Promise<{ status: "ok" | "error"; error?: string }>) {
    if (!inTauri()) return;
    const res = await call();
    this.error = res.status === "error" ? (res.error ?? "Failed") : null;
  }

  setShortcut(value: string | null) {
    this.shortcut = value;
    if (value) return this.apply(() => commands.changeBinding("transcribe", toBinding(value)));
  }
  setActivation(value: ShortcutActivation) {
    this.activation = value;
    return this.apply(() => commands.changeShortcutActivationSetting(value));
  }
  setOverlayStyle(value: OverlayStyle) {
    this.overlayStyle = value;
    return this.apply(() => commands.changeOverlayStyleSetting(value));
  }
  setMute(value: boolean) {
    this.muteWhileRecording = value;
    return this.apply(() => commands.changeMuteWhileRecordingSetting(value));
  }
  setCleanup(value: boolean) {
    this.cleanupWithLlm = value;
    return this.apply(() => commands.changePostProcessEnabledSetting(value));
  }
}

export const dictation = new DictationSettings();
