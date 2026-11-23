import { convertFileSrc } from "@tauri-apps/api/core";
import { commands, type ModelStatus, type SpeechClip, type VoiceGenRequest } from "../bindings";
import { inTauri } from "./env";

/** Settings a speech model can respond to. `null` means "let the model decide". */
export type VoiceSettings = {
  voice: string | null;
  language: string | null;
  speed: number;
  seed: number | null;
  temperature: number | null;
  design: string;
  referencePath: string | null;
  referenceText: string;
  flowSteps: number | null;
  cfgScale: number | null;
};

const DEFAULTS: VoiceSettings = {
  voice: null,
  language: null,
  speed: 1,
  seed: null,
  temperature: null,
  design: "",
  referencePath: null,
  referenceText: "",
  flowSteps: null,
  cfgScale: null,
};

/** Voice generation: pick a speech model, load it, change its settings, and hear a line. */
class VoiceGenStore {
  /** What the Voice page speaks with: "natural" (read-aloud voices) or a speech model id. */
  source = $state<string>(
    typeof localStorage !== "undefined" ? (localStorage.getItem("airlock-voice-source") ?? "natural") : "natural",
  );
  /** The model being tried out in the studio. */
  modelId = $state<string | null>(null);
  loaded = $state<string | null>(null);
  voices = $state<string[]>([]);
  loading = $state(false);
  making = $state(false);
  error = $state<string | null>(null);
  text = $state("Hello! This is a voice made on this computer.");
  settings = $state<VoiceSettings>({ ...DEFAULTS });
  clip = $state<SpeechClip | null>(null);
  private audio: HTMLAudioElement | null = null;

  async init() {
    if (!inTauri()) return;
    const res = await commands.voiceGenStatus();
    if (res.status === "ok") {
      this.loaded = res.data.loaded;
      this.voices = res.data.voices;
      if (!this.modelId) this.modelId = res.data.loaded;
    }
  }

  setSource(value: string) {
    this.source = value;
    localStorage.setItem("airlock-voice-source", value);
  }

  /** Switches the studio to another model, with its own fresh settings. */
  select(model: ModelStatus) {
    if (this.modelId === model.id) return;
    this.modelId = model.id;
    this.clip = null;
    this.error = null;
    this.settings = {
      ...DEFAULTS,
      voice: model.voices[0]?.id ?? null,
      language: model.languages.includes("en") ? "en" : (model.languages[0] ?? null),
    };
  }

  async load(id: string) {
    this.loading = true;
    this.error = null;
    const res = await commands.voiceGenLoad(id);
    this.loading = false;
    if (res.status === "error") {
      this.error = res.error;
      return false;
    }
    this.loaded = res.data.loaded;
    this.voices = res.data.voices;
    if (!this.settings.voice && res.data.voices.length > 0) this.settings.voice = res.data.voices[0];
    return true;
  }

  async unload() {
    await commands.voiceGenUnload();
    this.loaded = null;
    this.voices = [];
  }

  async say() {
    const id = this.modelId;
    if (!id || this.making) return;
    this.making = true;
    this.error = null;
    this.stop();
    const s = this.settings;
    const req: VoiceGenRequest = {
      id,
      text: this.text,
      voice: s.voice,
      language: s.language,
      speed: s.speed,
      seed: s.seed,
      temperature: s.temperature,
      design: s.design.trim() || null,
      reference_path: s.referencePath,
      reference_text: s.referenceText.trim() || null,
      flow_steps: s.flowSteps,
      cfg_scale: s.cfgScale,
    };
    const res = await commands.voiceGenSay(req);
    this.making = false;
    if (res.status === "error") {
      this.error = res.error;
      return;
    }
    this.clip = res.data;
    this.play();
  }

  play() {
    if (!this.clip) return;
    this.stop();
    this.audio = new Audio(convertFileSrc(this.clip.path));
    void this.audio.play().catch(() => (this.error = "The clip could not be played."));
  }

  stop() {
    this.audio?.pause();
    this.audio = null;
  }
}

export const voicegen = new VoiceGenStore();
