import { Channel } from "@tauri-apps/api/core";
import {
  commands,
  type DownloadEvent,
  type HfFile,
  type HfModel,
  type HfRepo,
  type SystemMemory,
} from "../bindings";
import { inTauri } from "./env";
import { llm } from "./llm.svelte";

export type ExploreTask = "chat" | "vision";
export type ExploreSort = "trendingScore" | "downloads" | "likes" | "lastModified";

export type RepoDownload = {
  phase: "downloading" | "verifying";
  downloaded: number;
  total: number;
  bytesPerSecond: number;
};

/** Browses Hugging Face for GGUF models the local engine can run, and downloads a chosen file. */
class ExploreStore {
  query = $state("");
  task = $state<ExploreTask>("chat");
  sort = $state<ExploreSort>("trendingScore");
  results = $state<HfModel[]>([]);
  searching = $state(false);
  searched = $state(false);
  error = $state<string | null>(null);

  /** The repo whose files are open in the dialog. */
  selected = $state<HfRepo | null>(null);
  loadingRepo = $state(false);
  repoError = $state<string | null>(null);
  memory = $state<SystemMemory | null>(null);
  download = $state<RepoDownload | null>(null);
  downloadError = $state<string | null>(null);

  private timer: ReturnType<typeof setTimeout> | undefined;
  private runId = 0;

  /** Search after a short pause in typing, so each keystroke is not a network call. */
  schedule() {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.search(), 400);
  }

  async search() {
    if (!inTauri()) return;
    const id = ++this.runId;
    this.searching = true;
    this.error = null;
    const res = await commands.llmHfSearch(this.query.trim(), this.task, this.sort);
    if (id !== this.runId) return; // a newer search replaced this one
    this.searching = false;
    this.searched = true;
    if (res.status === "ok") this.results = res.data;
    else {
      this.results = [];
      this.error = res.error;
    }
  }

  /** The model card text of the open repo: null until asked for, empty when it has none. */
  readme = $state<string | null>(null);
  readmeLoading = $state(false);
  readmeError = $state<string | null>(null);

  /** Fetches the model card once per open repo; called when its tab is first shown. */
  async loadReadme() {
    const id = this.selected?.model.id;
    if (!id || this.readme !== null || this.readmeLoading) return;
    this.readmeLoading = true;
    this.readmeError = null;
    const res = await commands.llmHfReadme(id);
    if (this.selected?.model.id !== id) return;
    this.readmeLoading = false;
    if (res.status === "ok") this.readme = res.data;
    else this.readmeError = res.error;
  }

  async open(model: HfModel) {
    this.readme = null;
    this.readmeLoading = false;
    this.readmeError = null;
    this.selected = { model, files: [] };
    this.loadingRepo = true;
    this.repoError = null;
    this.download = null;
    this.downloadError = null;
    const [repo, mem] = await Promise.all([commands.llmHfFiles(model.id), commands.llmSystemMemory()]);
    this.loadingRepo = false;
    this.memory = mem;
    if (repo.status === "ok") this.selected = repo.data;
    else this.repoError = repo.error;
  }

  close() {
    this.selected = null;
  }

  /** Downloads `file` (and its vision projector, if any); resolves true once the model is added. */
  async fetch(repo: string, file: HfFile, projector: HfFile | null): Promise<boolean> {
    this.downloadError = null;
    const total = file.size_bytes + (projector?.size_bytes ?? 0);
    this.download = { phase: "downloading", downloaded: 0, total, bytesPerSecond: 0 };
    let cancelled = false;
    const channel = new Channel<DownloadEvent>();
    channel.onmessage = (e) => {
      if (e.type === "progress") {
        this.download = {
          phase: "downloading",
          downloaded: e.downloaded,
          total: e.total,
          bytesPerSecond: e.bytes_per_second,
        };
      } else if (e.type === "verifying" && this.download) {
        this.download = { ...this.download, phase: "verifying" };
      } else if (e.type === "cancelled") {
        cancelled = true;
      } else if (e.type === "error") {
        this.downloadError = e.message;
      }
    };
    const res = await commands.llmHfDownload(repo, file.name, projector?.name ?? null, channel);
    this.download = null;
    if (res.status === "error") this.downloadError = res.error;
    if (this.downloadError || cancelled) return false;
    await llm.refresh();
    return true;
  }

  cancel(repo: string, file: HfFile) {
    return commands.llmDownloadCancel(`hf:${repo}/${file.name}`);
  }
}

export const explore = new ExploreStore();
