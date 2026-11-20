import { chunkForSpeech, plainForSpeech } from "./speak-text";

/**
 * Read-aloud for replies, using the webview's speech synthesis.
 * Decision (product): only voices that run on this computer are used. Some systems offer
 * "natural" voices that stream from the cloud, which would break the promise that replies
 * never leave the machine, so those are skipped.
 */
class Speaker {
  /** Id of the message being read, or null. */
  speakingId = $state<string | null>(null);
  private voices: SpeechSynthesisVoice[] = [];
  private run = 0;

  get available() {
    return typeof speechSynthesis !== "undefined" && this.local().length > 0;
  }

  private local() {
    if (typeof speechSynthesis === "undefined") return [];
    this.voices = speechSynthesis.getVoices().filter((v) => v.localService);
    return this.voices;
  }

  /** Voices load asynchronously on some systems; call once at startup. */
  init() {
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
    const chunks = chunkForSpeech(plainForSpeech(markdown));
    if (chunks.length === 0) return;
    const voice = this.pick();
    const run = ++this.run;
    this.speakingId = id;
    chunks.forEach((text, i) => {
      const u = new SpeechSynthesisUtterance(text);
      if (voice) {
        u.voice = voice;
        u.lang = voice.lang;
      }
      const done = () => {
        if (run === this.run && i === chunks.length - 1) this.speakingId = null;
      };
      u.onend = done;
      u.onerror = done;
      speechSynthesis.speak(u);
    });
  }

  stop() {
    this.run++;
    if (typeof speechSynthesis !== "undefined") speechSynthesis.cancel();
    this.speakingId = null;
  }
}

export const speaker = new Speaker();
