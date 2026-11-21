import { Channel } from "@tauri-apps/api/core";
import {
  commands,
  type DownloadEvent,
  type ImageEngineStatus,
  type ImageEvent,
  type ImageInfo,
  type ImageModelStatus,
} from "../bindings";
import { inTauri } from "./env";

export type Progress = { phase: "downloading" | "verifying"; downloaded: number; total: number; bytesPerSecond: number };
export type Run = { phase: "queued" | "starting" | "making"; step: number; total: number; prompt: string; width: number; height: number; log: string[]; startedAt: number };

/** Image generation: engine, model downloads, the one running job and the gallery. */
class ImageStore {
  loaded = $state(false);
  error = $state<string | null>(null);
  engines = $state<ImageEngineStatus[]>([]);
  models = $state<ImageModelStatus[]>([]);
  gallery = $state<ImageInfo[]>([]);
  downloads = $state<Record<string, Progress>>({});
  run = $state<Run | null>(null);
  /** The picture just made, shown large on the Create tab until the next one starts. */
  last = $state<ImageInfo | null>(null);
  /** Which stage the latest run failed in, so the steps can show where it stopped. */
  failedPhase = $state<Run["phase"] | null>(null);
  settingUp = $state(false);
  /** Model picked in the composer; empty means the first downloaded one. */
  choice = $state("");

  get supported() {
    return this.engines.length > 0;
  }

  get hasEngine() {
    return this.engines.some((e) => e.installed);
  }

  get downloaded() {
    return this.models.filter((m) => m.downloaded);
  }

  get ready() {
    return this.hasEngine && this.downloaded.length > 0;
  }

  get model() {
    return this.downloaded.find((m) => m.id === this.choice) ?? this.downloaded[0] ?? null;
  }

  get recommendedEngine() {
    return this.engines.find((e) => e.recommended) ?? this.engines[0];
  }

  get recommendedModel() {
    return this.models.find((m) => m.recommended) ?? this.models[0];
  }

  get busy() {
    return this.run !== null;
  }

  async init() {
    if (this.loaded) return;
    if (!inTauri()) {
      this.loaded = true;
      return;
    }
    await Promise.all([this.refresh(), this.refreshGallery()]);
    this.loaded = true;
  }

  async refresh() {
    const [engines, models] = await Promise.all([commands.imageEngines(), commands.imageModels()]);
    if (engines.status === "ok") this.engines = engines.data;
    else this.error = engines.error;
    if (models.status === "ok") this.models = models.data;
    else this.error = models.error;
  }

  async refreshGallery() {
    const res = await commands.imageList();
    if (res.status === "ok") this.gallery = res.data;
    else this.error = res.error;
  }

  private track(key: string) {
    const channel = new Channel<DownloadEvent>();
    channel.onmessage = (e) => {
      if (e.type === "progress") {
        this.downloads[key] = {
          phase: "downloading",
          downloaded: e.downloaded,
          total: e.total,
          bytesPerSecond: e.bytes_per_second,
        };
      } else if (e.type === "verifying") {
        const cur = this.downloads[key];
        if (cur) cur.phase = "verifying";
      } else if (e.type === "error") {
        this.error = e.message;
      }
    };
    this.downloads[key] = { phase: "downloading", downloaded: 0, total: 0, bytesPerSecond: 0 };
    return channel;
  }

  async installEngine(backend: ImageEngineStatus["backend"]) {
    this.error = null;
    const key = `engine:${backend}`;
    const res = await commands.imageInstallEngine(backend, this.track(key));
    if (res.status === "error") this.error = res.error;
    delete this.downloads[key];
    await this.refresh();
  }

  async download(id: string) {
    this.error = null;
    const res = await commands.imageDownload(id, this.track(id));
    if (res.status === "error") this.error = res.error;
    delete this.downloads[id];
    await this.refresh();
  }

  /** `engine:vulkan` style keys map to the engine folder name the backend tracks. */
  cancelDownload(key: string) {
    return commands.imageDownloadCancel(key.startsWith("engine:") ? `engine:sd-${key.slice(7)}` : key);
  }

  async removeModel(id: string) {
    const res = await commands.imageDeleteModel(id);
    if (res.status === "error") this.error = res.error;
    await this.refresh();
  }

  /** One button: install the engine and the recommended model, whichever is missing. */
  async runSetup() {
    this.settingUp = true;
    try {
      const engine = this.recommendedEngine;
      if (engine && !this.hasEngine) await this.installEngine(engine.backend);
      const model = this.recommendedModel;
      if (this.hasEngine && model && !model.downloaded) await this.download(model.id);
    } finally {
      this.settingUp = false;
    }
  }

  async generate(prompt: string, width: number, height: number) {
    const model = this.model;
    if (!model || this.run) return;
    this.error = null;
    this.last = null;
    this.failedPhase = null;
    const startedAt = Date.now();
    const log: string[] = [];
    const run = (phase: Run["phase"], step = 0, total = 0): Run => ({ phase, step, total, prompt, width, height, log, startedAt });
    this.run = run("starting");
    const channel = new Channel<ImageEvent>();
    channel.onmessage = (e) => {
      if (e.type === "queued") this.run = run("queued");
      else if (e.type === "starting") this.run = run("starting");
      else if (e.type === "log") {
        log.push(e.line);
        if (log.length > 40) log.shift();
        if (this.run) this.run = { ...this.run, log: [...log] };
      } else if (e.type === "progress") this.run = run("making", e.step, e.total);
      else if (e.type === "done") {
        this.last = e.image;
        this.gallery = [e.image, ...this.gallery];
      } else if (e.type === "error") {
        this.failedPhase = this.run?.phase ?? "starting";
        this.error = e.message;
      }
    };
    try {
      await commands.imageGenerate({ prompt, model: model.id, width, height, steps: null, seed: null }, channel);
    } finally {
      this.run = null;
    }
  }

  /** Resolves once no image is being made, so chat can queue behind it. */
  idle(): Promise<void> {
    return new Promise((resolve) => {
      const check = () => (this.run ? setTimeout(check, 500) : resolve());
      check();
    });
  }

  cancel() {
    return commands.imageCancel();
  }

  async remove(id: string) {
    const res = await commands.imageDelete(id);
    if (res.status === "error") this.error = res.error;
    this.gallery = this.gallery.filter((i) => i.id !== id);
    if (this.last?.id === id) this.last = null;
  }
}

export const images = new ImageStore();
