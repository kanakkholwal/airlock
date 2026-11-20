import { listen } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type Notice,
  type StreamPhase,
  type StreamPhaseEvent,
  type StreamTextEvent,
  type StreamWorkKind,
  type Theme,
} from "../../bindings";
import { applyTheme } from "../theme";

export type OverlayState =
  | "recording"
  | "streaming"
  | "transcribing"
  | "processing"
  | "notice"
  | "live_notice";

export const isNoticeState = (s: OverlayState) => s === "notice" || s === "live_notice";

/** Reactive bars in the waveform. Mic levels arrive as 16 FFT buckets; the first N are shown. */
export const WAVE_BARS = 9;

// A model load is only called out in the live preview once it has run this long. Warm loads
// finish well under it, so the common case never flashes a notice while the user speaks.
const SLOW_MODEL_LOAD_MS = 2000;

// Matches the delay in `hide_recording_overlay`, so a notice can fade out.
const OVERLAY_FADE_MS = 300;

const emptyText = (): StreamTextEvent => ({ committed: "", tentative: "" });
const silence = () => Array<number>(16).fill(0);

type ModelStateEvent = { event_type: string };

/**
 * State of the recording overlay window. Rust drives it with events; the component only draws.
 * Provenance: Handy src/overlay/RecordingOverlay.tsx, same state machine in Svelte.
 */
export class OverlaySession {
  isVisible = $state(false);
  state = $state<OverlayState>("recording");
  /** `Stream::play()` returning does not mean samples are flowing; stay "arming" until they do. */
  captureReady = $state(false);
  modelLoading = $state(false);
  modelLoadSlow = $state(false);
  /** Latched per live session so the panel does not collapse and reopen when text arrives. */
  loadNoticeShown = $state(false);
  levels = $state<number[]>(Array<number>(WAVE_BARS).fill(0));
  streamText = $state<StreamTextEvent>(emptyText());
  phase = $state<StreamPhase>("listening");
  workKind = $state<StreamWorkKind>("transcribing");
  elapsed = $state(0);
  /** Bumped per streaming session so the live card remounts fresh. */
  session = $state(0);
  position = $state<"top" | "bottom">("bottom");
  notice = $state<Notice | null>(null);

  private smoothed = silence();
  private modelTimer: ReturnType<typeof setTimeout> | undefined;
  private fadeTimer: ReturnType<typeof setTimeout> | undefined;

  /** Starts listening to the backend; returns the function that stops it. */
  async start(): Promise<() => void> {
    const stops = await Promise.all([
      listen<Notice>("overlay-notice", (e) => {
        clearTimeout(this.fadeTimer);
        this.notice = e.payload;
      }),
      listen<OverlayState>("show-overlay", (e) => void this.show(e.payload)),
      listen("hide-overlay", () => this.hide()),
      listen("recording-ready", () => {
        this.elapsed = 0;
        this.captureReady = true;
      }),
      listen<number[]>("mic-level", (e) => {
        // Exponential smoothing across the buckets, then the first N bars.
        this.smoothed = this.smoothed.map((prev, i) => prev * 0.7 + (e.payload[i] || 0) * 0.3);
        this.levels = this.smoothed.slice(0, WAVE_BARS);
      }),
      events.streamTextEvent.listen((e) => (this.streamText = e.payload)),
      events.streamPhaseEvent.listen((e) => {
        const p: StreamPhaseEvent = e.payload;
        this.phase = p.phase;
        if (p.kind) this.workKind = p.kind;
      }),
      listen<ModelStateEvent>("model-state-changed", (e) => this.modelEvent(e.payload.event_type)),
      // A separate webview from the app window, so it follows theme changes itself.
      listen<Theme>("theme-changed", (e) => applyTheme(e.payload)),
    ]);
    return () => {
      for (const stop of stops) stop();
      clearTimeout(this.modelTimer);
      clearTimeout(this.fadeTimer);
    };
  }

  private async show(next: OverlayState) {
    clearTimeout(this.fadeTimer);
    if (!isNoticeState(next)) this.notice = null;
    // Reset synchronously, before any await: a fast microphone can emit recording-ready
    // while the settings read is in flight, and resetting after it would leave us arming.
    if (next === "recording" || next === "streaming") {
      this.captureReady = false;
      this.smoothed = silence();
      this.levels = Array<number>(WAVE_BARS).fill(0);
      this.streamText = emptyText();
    }
    try {
      const settings = await commands.getAppSettings();
      if (settings.status === "ok") {
        this.position = settings.data.overlay_position === "top" ? "top" : "bottom";
      }
    } catch {
      // Keep the previous placement if settings cannot be read.
    }
    this.state = next;
    if (next === "streaming") {
      this.phase = "listening";
      this.workKind = "transcribing";
      this.elapsed = 0;
      this.loadNoticeShown = false;
      this.session += 1;
    }
    this.isVisible = true;
  }

  private hide() {
    this.isVisible = false;
    this.captureReady = false;
    // A notice keeps rendering while it fades; clearing the transcript afterwards stops a later
    // notice from showing an old session's text.
    clearTimeout(this.fadeTimer);
    this.fadeTimer = setTimeout(() => {
      this.notice = null;
      this.streamText = emptyText();
    }, OVERLAY_FADE_MS);
  }

  // The backend ends every `loading_started` with exactly one completed or failed, so only
  // those end a load. Other events (the idle watcher's `unloaded`) are not ordered against it.
  private modelEvent(type: string) {
    if (type === "loading_started") {
      clearTimeout(this.modelTimer);
      this.modelLoading = true;
      this.modelLoadSlow = false;
      this.modelTimer = setTimeout(() => (this.modelLoadSlow = true), SLOW_MODEL_LOAD_MS);
    } else if (type === "loading_completed" || type === "loading_failed") {
      clearTimeout(this.modelTimer);
      this.modelLoading = false;
      this.modelLoadSlow = false;
    }
  }
}
