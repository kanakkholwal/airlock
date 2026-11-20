import {
  commands,
  type AutoSubmitKey,
  type ClipboardHandling,
  type OverlayPosition,
  type OverlayStyle,
  type PasteMethod,
  type ShortcutActivation,
} from "../bindings";

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
  overlayPosition = $state<OverlayPosition>("bottom");
  pasteMethod = $state<PasteMethod>("ctrl_v");
  clipboardHandling = $state<ClipboardHandling>("dont_modify");
  autoSubmit = $state(false);
  autoSubmitKey = $state<AutoSubmitKey>("enter");
  appendTrailingSpace = $state(false);
  fillerWords = $state(true);
  customWords = $state<string[]>([]);
  vad = $state(true);
  longFormChunking = $state(true);

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
    this.overlayPosition = s.overlay_position ?? this.overlayPosition;
    this.pasteMethod = s.paste_method ?? this.pasteMethod;
    this.clipboardHandling = s.clipboard_handling ?? this.clipboardHandling;
    this.autoSubmit = s.auto_submit ?? false;
    this.autoSubmitKey = s.auto_submit_key ?? this.autoSubmitKey;
    this.appendTrailingSpace = s.append_trailing_space ?? false;
    this.fillerWords = s.filler_word_removal_enabled ?? true;
    this.customWords = s.custom_words ?? [];
    this.vad = s.vad_enabled ?? true;
    this.longFormChunking = s.long_form_chunking ?? true;
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
  setOverlayPosition(v: OverlayPosition) {
    this.overlayPosition = v;
    return this.apply(() => commands.changeOverlayPositionSetting(v));
  }
  setPasteMethod(v: PasteMethod) {
    this.pasteMethod = v;
    return this.apply(() => commands.changePasteMethodSetting(v));
  }
  setClipboardHandling(v: ClipboardHandling) {
    this.clipboardHandling = v;
    return this.apply(() => commands.changeClipboardHandlingSetting(v));
  }
  setAutoSubmit(v: boolean) {
    this.autoSubmit = v;
    return this.apply(() => commands.changeAutoSubmitSetting(v));
  }
  setAutoSubmitKey(v: AutoSubmitKey) {
    this.autoSubmitKey = v;
    return this.apply(() => commands.changeAutoSubmitKeySetting(v));
  }
  setTrailingSpace(v: boolean) {
    this.appendTrailingSpace = v;
    return this.apply(() => commands.changeAppendTrailingSpaceSetting(v));
  }
  setFillerWords(v: boolean) {
    this.fillerWords = v;
    return this.apply(() => commands.changeFillerWordRemovalEnabledSetting(v));
  }
  setCustomWords(words: string[]) {
    this.customWords = words;
    return this.apply(() => commands.updateCustomWords(words));
  }
  setVad(v: boolean) {
    this.vad = v;
    return this.apply(() => commands.changeVadEnabledSetting(v));
  }
  setLongFormChunking(v: boolean) {
    this.longFormChunking = v;
    return this.apply(() => commands.changeLongFormChunkingSetting(v));
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
