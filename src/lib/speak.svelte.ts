import { convertFileSrc } from "@tauri-apps/api/core";
import { chunkForSpeech, plainForSpeech } from "./speak-text";
import { tts } from "./tts.svelte";

/**
 * Read-aloud for replies. Natural voices (Kokoro) are used once their voice pack is installed;
 * until then the webview's speech synthesis reads it.
 * Decision (product): only voices that run on this computer are used. Some systems offer
 * "natural" voices that stream from the cloud, which would break the promise that replies
 * never leave the machine, so those are skipped.
 */
class Speaker {
  /** Id of the message being read, or null. */
  speakingId = $state<string | null>(null);
  /** True from the click until the first sound plays, so the button can show it is working. */
  preparing = $state(false);
  paused = $state(false);
  /** Which voice engine is reading, so the controls can say so. */
  engine = $state<"natural" | "system" | null>(null);
  /** Sentence being read, from 1, and how many there are. */
  position = $state({ at: 0, of: 0 });
  error = $state<string | null>(null);
  private voices: SpeechSynthesisVoice[] = [];
  private run = 0;
  private audio: HTMLAudioElement | null = null;
  private cancelPlay: (() => void) | null = null;

  get available() {
    return tts.ready || (typeof speechSynthesis !== "undefined" && this.local().length > 0);
  }

  private local() {
    if (typeof speechSynthesis === "undefined") return [];
    this.voices = speechSynthesis.getVoices().filter((v) => v.localService);
    return this.voices;
  }

  /** Voices load asynchronously on some systems; call once at startup. */
  init() {
    void tts.init();
    if (typeof speechSynthesis === "undefined") return;
    this.local();
    speechSynthesis.addEventListener("voiceschanged", () => this.local());
  }

  private pick() {
    const lang = navigator.language.toLowerCase();
    const voices = this.local();
    return (
      voices.find((v) => v.lang.toLowerCase() === lang) ??
      voices.find((v) => v.lang.toLowerCase().startsWith(lang.split("-")[0])) ??
      voices.find((v) => v.default) ??
      voices[0]
    );
  }

  speak(markdown: string, id: string) {
    this.stop();
    this.error = null;
    const text = plainForSpeech(markdown);
    const run = ++this.run;
    if (tts.ready) {
      const chunks = chunkForSpeech(text, 300);
      if (chunks.length === 0) return;
      this.speakingId = id;
      this.preparing = true;
      this.engine = "natural";
      this.position = { at: 0, of: chunks.length };
      void this.readNatural(chunks, run);
      return;
    }
    this.readSystem(chunkForSpeech(text), id, run);
  }

  /** Makes the next clip while the current one plays, so there is no gap between sentences. */
  private async readNatural(chunks: string[], run: number) {
    try {
      let upcoming = tts.synthesize(chunks[0]);
      for (let i = 0; i < chunks.length; i++) {
        const current = upcoming;
        upcoming = i + 1 < chunks.length ? tts.synthesize(chunks[i + 1]) : Promise.resolve(null as never);
        // A clip that is never awaited must not raise an unhandled rejection after a stop.
        upcoming.catch(() => {});
        const clip = await current;
        if (run !== this.run) return;
        this.position = { at: i + 1, of: chunks.length };
        await this.play(clip.path, run);
        if (run !== this.run) return;
      }
    } catch (e) {
      if (run === this.run) this.error = e instanceof Error ? e.message : String(e);
    }
    if (run === this.run) this.reset();
  }

  private reset() {
    this.speakingId = null;
    this.preparing = false;
    this.paused = false;
    this.engine = null;
    this.position = { at: 0, of: 0 };
  }

  pause() {
    if (!this.speakingId || this.paused) return;
    this.paused = true;
    if (this.engine === "natural") this.audio?.pause();
    else speechSynthesis.pause();
  }

  resume() {
    if (!this.speakingId || !this.paused) return;
    this.paused = false;
    if (this.engine === "natural") void this.audio?.play();
    else speechSynthesis.resume();
  }

  private play(path: string, run: number) {
    return new Promise<void>((resolve) => {
      const audio = new Audio(convertFileSrc(path));
      this.audio = audio;
      const finish = () => resolve();
      this.cancelPlay = finish;
      audio.onended = finish;
      audio.onerror = finish;
      audio.play().then(
        () => {
          if (run === this.run) this.preparing = false;
        },
        finish,
      );
    });
  }

  private readSystem(chunks: string[], id: string, run: number) {
    if (chunks.length === 0) return;
    const voice = this.pick();
    this.speakingId = id;
    this.engine = "system";
    this.position = { at: 1, of: chunks.length };
    chunks.forEach((text, i) => {
      const u = new SpeechSynthesisUtterance(text);
      if (voice) {
        u.voice = voice;
        u.lang = voice.lang;
      }
      const done = () => {
        if (run === this.run && i === chunks.length - 1) this.reset();
      };
      u.onstart = () => {
        if (run === this.run) this.position = { at: i + 1, of: chunks.length };
      };
      u.onend = done;
      u.onerror = done;
      speechSynthesis.speak(u);
    });
  }

  stop() {
    this.run++;
    this.audio?.pause();
    this.audio = null;
    // A paused clip never ends by itself, so release whoever is waiting for it.
    this.cancelPlay?.();
    this.cancelPlay = null;
    if (typeof speechSynthesis !== "undefined") speechSynthesis.cancel();
    this.reset();
  }
}

export const speaker = new Speaker();
