import { Channel, convertFileSrc } from "@tauri-apps/api/core";
import { commands, type DownloadEvent, type SpeechClip, type TtsStatus } from "../bindings";
import { inTauri } from "./env";

const VOICE_KEY = "airlock-tts-voice";
const SPEED_KEY = "airlock-tts-speed";

export type Progress = { phase: "downloading" | "verifying"; downloaded: number; total: number; bytesPerSecond: number };

/** Natural-sounding read-aloud voices (Kokoro), their download, and the voice and speed chosen. */
class TtsStore {
  status = $state<TtsStatus | null>(null);
  error = $state<string | null>(null);
  progress = $state<Progress | null>(null);
  voice = $state(3);
  speed = $state(1);
  /** True while a preview clip is being made or played. */
  previewing = $state(false);
  /** Clips being made right now. The voice engine starts for each one, so this is its busy state. */
  making = $state(0);
  private audio: HTMLAudioElement | null = null;

  get supported() {
    return this.status?.supported ?? false;
  }

  get ready() {
    return this.status?.ready ?? false;
  }

  /** The voice model in use, such as "Kokoro natural voices". */
  get modelName() {
    return this.status?.model_name ?? "Voice model";
  }

  /** The chosen voice as people read it, such as "Heart" or "Daniel". */
  get voiceName() {
    return this.status?.voices.find((v) => v.id === this.voice)?.name ?? "Voice";
  }

  get installing() {
    return this.progress !== null;
  }

  async init() {
    if (!inTauri() || this.status) return;
    const v = Number(localStorage.getItem(VOICE_KEY));
    const s = Number(localStorage.getItem(SPEED_KEY));
    if (Number.isFinite(v) && localStorage.getItem(VOICE_KEY) !== null) this.voice = v;
    if (Number.isFinite(s) && s >= 0.5 && s <= 2 && localStorage.getItem(SPEED_KEY) !== null) this.speed = s;
    await this.refresh();
    // Re-read after the await: TypeScript still thinks it is null from the guard above.
    const loaded = this.status as TtsStatus | null;
    if (loaded && !loaded.voices.some((x) => x.id === this.voice)) this.voice = loaded.default_voice;
  }

  async refresh() {
    const res = await commands.ttsStatus();
    if (res.status === "ok") this.status = res.data;
    else this.error = res.error;
  }

  setVoice(id: number) {
    this.voice = id;
    localStorage.setItem(VOICE_KEY, String(id));
  }

  setSpeed(value: number) {
    this.speed = value;
    localStorage.setItem(SPEED_KEY, String(value));
  }

  async install() {
    if (this.installing) return;
    this.error = null;
    const channel = new Channel<DownloadEvent>();
    channel.onmessage = (e) => {
      if (e.type === "progress") {
        this.progress = {
          phase: "downloading",
          downloaded: e.downloaded,
          total: e.total,
          bytesPerSecond: e.bytes_per_second,
        };
      } else if (e.type === "verifying") {
        if (this.progress) this.progress = { ...this.progress, phase: "verifying" };
      } else if (e.type === "error") {
        this.error = e.message;
      }
    };
    this.progress = { phase: "downloading", downloaded: 0, total: 0, bytesPerSecond: 0 };
    const res = await commands.ttsInstall(channel);
    if (res.status === "error") this.error = res.error;
    this.progress = null;
    await this.refresh();
  }

  cancelInstall() {
    return commands.ttsInstallCancel();
  }

  async remove() {
    this.stopPreview();
    const res = await commands.ttsRemove();
    if (res.status === "error") this.error = res.error;
    await this.refresh();
  }

  /** Makes one clip of speech; the caller plays it. */
  async synthesize(text: string, voice = this.voice, speed = this.speed): Promise<SpeechClip> {
    this.making++;
    try {
      const res = await commands.ttsSynthesize({ text, voice, speed });
      if (res.status === "error") throw new Error(res.error);
      return res.data;
    } finally {
      this.making--;
    }
  }

  /** Plays a short sample of the chosen voice, so people hear it before choosing. */
  async preview(voice = this.voice) {
    this.stopPreview();
    this.previewing = true;
    try {
      const clip = await this.synthesize("Hi, I am reading this out loud, right here on your computer.", voice);
      if (!this.previewing) return;
      const audio = new Audio(convertFileSrc(clip.path));
      this.audio = audio;
      audio.onended = () => (this.previewing = false);
      audio.onerror = () => (this.previewing = false);
      await audio.play();
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
      this.previewing = false;
    }
  }

  stopPreview() {
    this.previewing = false;
    this.audio?.pause();
    this.audio = null;
  }
}

export const tts = new TtsStore();
