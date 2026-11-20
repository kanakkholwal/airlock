import { Channel } from "@tauri-apps/api/core";
import {
  commands,
  events,
  type Backend,
  type ChatEvent,
  type DownloadEvent,
  type EngineState,
  type MemoryCheck,
  type ModelStatus,
  type SpeedResult,
} from "../bindings";
import { chat, type ChatMessage, type ChatTransport, type EngineState as ChatEngine } from "./chat.svelte";
import type { SpeedTarget } from "./speed";

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type DownloadProgress = {
  phase: "downloading" | "verifying";
  downloaded: number;
  total: number;
  bytesPerSecond: number;
};

const toChatEngine = (s: EngineState): ChatEngine =>
  s.state === "ready" ? "ready" : s.state === "loading" ? "loading" : s.state === "failed" ? "error" : "unavailable";

const textOf = (m: ChatMessage) =>
  m.parts.flatMap((p) => (p.type === "text" ? [p.text] : [])).join("");

const transport: ChatTransport = async (history, onText, signal) => {
  const channel = new Channel<ChatEvent>();
  let failure: string | null = null;
  channel.onmessage = (e) => {
    if (e.type === "text") onText(e.delta);
    else if (e.type === "error") failure = e.message;
  };
  const onAbort = () => void commands.chatCancel();
  signal.addEventListener("abort", onAbort);
  try {
    const res = await commands.chatStream(
      history.map((m) => ({ role: m.role, content: textOf(m) })),
      channel,
    );
    if (res.status === "error") throw new Error(res.error);
    if (failure) throw new Error(failure);
  } finally {
    signal.removeEventListener("abort", onAbort);
  }
};

class LlmStore {
  engine = $state<EngineState>({ state: "stopped" });
  models = $state<ModelStatus[]>([]);
  downloads = $state<Record<string, DownloadProgress>>({});
  error = $state<string | null>(null);
  private started = false;

  async init() {
    if (this.started || !inTauri()) return;
    this.started = true;
    await events.llmStateChanged.listen((e) => this.setEngine(e.payload));
    const status = await commands.llmStatus();
    if (status.status === "ok") this.setEngine(status.data);
    await this.refresh();
  }

  private setEngine(state: EngineState) {
    this.engine = state;
    chat.attach(state.state === "ready" ? transport : null, toChatEngine(state));
  }

  async refresh() {
    const res = await commands.llmCatalog();
    if (res.status === "ok") this.models = res.data;
    else this.error = res.error;
  }

  async download(id: string) {
    this.error = null;
    const channel = new Channel<DownloadEvent>();
    channel.onmessage = (e) => {
      if (e.type === "progress") {
        this.downloads[id] = {
          phase: "downloading",
          downloaded: e.downloaded,
          total: e.total,
          bytesPerSecond: e.bytes_per_second,
        };
      } else if (e.type === "verifying") {
        const cur = this.downloads[id];
        if (cur) cur.phase = "verifying";
      } else if (e.type === "error") {
        this.error = e.message;
      }
    };
    this.downloads[id] = { phase: "downloading", downloaded: 0, total: 0, bytesPerSecond: 0 };
    const res = await commands.llmDownload(id, channel);
    if (res.status === "error") this.error = res.error;
    delete this.downloads[id];
    await this.refresh();
  }

  cancelDownload(id: string) {
    return commands.llmDownloadCancel(id);
  }

  async remove(id: string) {
    const res = await commands.llmDeleteModel(id);
    if (res.status === "error") this.error = res.error;
    await this.refresh();
  }

  async importFile(path: string) {
    const res = await commands.llmImportModel(path);
    if (res.status === "error") this.error = res.error;
    await this.refresh();
  }

  async start(id: string, backend: Backend = "auto") {
    this.error = null;
    const res = await commands.llmStartModel(id, backend);
    if (res.status === "error") this.error = res.error;
  }

  stop() {
    return commands.llmStop();
  }

  async memory(id: string): Promise<MemoryCheck | null> {
    const res = await commands.llmMemoryCheck(id);
    return res.status === "ok" ? res.data : null;
  }

  /** Speed test runner for the header dropdown; null until the engine is ready. */
  get speedRunner(): ((target: SpeedTarget) => Promise<SpeedResult[]>) | null {
    if (this.engine.state !== "ready") return null;
    return async (target) => {
      if (target !== "chat") throw new Error("Only the chat model can be measured for now");
      const res = await commands.speedTestChat();
      if (res.status === "error") throw new Error(res.error);
      return res.data;
    };
  }
}

export const llm = new LlmStore();
