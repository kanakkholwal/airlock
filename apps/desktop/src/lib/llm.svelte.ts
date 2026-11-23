import { Channel } from "@tauri-apps/api/core";
import {
  commands,
  events,
  type Backend,
  type Tuning,
  type ChatEvent,
  type DownloadEvent,
  type EngineStatus,
  type EngineState,
  type MemoryCheck,
  type ModelStatus,
  type SpeedResult,
  type StoragePaths,
} from "../bindings";
import {
  chat,
  imagesOf,
  referencesOf,
  textOf,
  type ChatMessage,
  type ChatTransport,
  type EngineState as ChatEngine,
  type Usage,
} from "./chat.svelte";
import { images } from "./images.svelte";
import { pickAutoModel } from "./model-pick";
import type { SpeedTarget } from "./speed";

const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type SetupStep = "engine" | "model" | "loading" | "done" | "failed";
export type SetupState = { step: SetupStep; message?: string };

export type DownloadProgress = {
  phase: "downloading" | "verifying";
  downloaded: number;
  total: number;
  bytesPerSecond: number;
};

const toChatEngine = (s: EngineState): ChatEngine =>
  s.state === "ready" ? "ready" : s.state === "loading" ? "loading" : s.state === "failed" ? "error" : "unavailable";

const DEFAULT_CONTEXT_TOKENS = 4096;

/**
 * Decision (privacy): a mention gives the model the item's id, title and a short preview, and
 * nothing more. To read further it must call a tool, which asks the person and passes the
 * privacy gateway. Cloud providers will filter these previews by the item's privacy flag first.
 */
function withReferences(text: string, refs: { id: string; title: string; kind: string; preview: string }[]) {
  if (refs.length === 0) return text;
  const lines = refs.map((r) =>
    r.kind === "file"
      ? `- file "${r.title}" at ${r.preview} (read it with read_text_file)`
      : `- ${r.id} (${r.kind}) "${r.title}": ${r.preview}`,
  );
  return `${text}\n\n[Attached. Library items are only previews: use read_item with the id to read one in full.]\n${lines.join("\n")}`;
}

// Heuristic: about 3 characters per token, and 70% of the window for the prompt so the reply
// has room. Oldest turns are dropped first; the newest message is always kept.
// Debt: move into Rust and use real token counts (see .local/ARCHITECTURE.md).
function fitHistory(history: ChatMessage[], contextTokens: number) {
  const budget = Math.floor(contextTokens * 3 * 0.7);
  const out: { role: string; content: string; images: string[] }[] = [];
  let used = 0;
  for (const m of [...history].reverse()) {
    const content = m.role === "user" ? withReferences(textOf(m), referencesOf(m)) : textOf(m);
    const images = m.role === "user" ? imagesOf(m) : [];
    if (!content && images.length === 0) continue;
    if (out.length > 0 && used + content.length > budget) break;
    out.unshift({ role: m.role, content, images });
    used += content.length;
  }
  return out;
}

const transport: ChatTransport = async (history, onDelta, signal, onTool) => {
  const channel = new Channel<ChatEvent>();
  let failure: string | null = null;
  const usage: Usage = { tokensPerSecond: null, promptTokens: null, completionTokens: null };
  channel.onmessage = (e) => {
    if (e.type === "text") onDelta("text", e.delta);
    else if (e.type === "reasoning") onDelta("reasoning", e.delta);
    else if (e.type === "tool_call")
      onTool({ type: "call", callId: e.call_id, name: e.name, arguments: e.arguments, title: e.title });
    else if (e.type === "tool_approval")
      onTool({ type: "approval", callId: e.call_id, name: e.name, title: e.title, risk: e.risk });
    else if (e.type === "tool_result")
      onTool({ type: "result", callId: e.call_id, output: e.output, isError: e.is_error });
    else if (e.type === "done") {
      usage.tokensPerSecond = e.tokens_per_second;
      usage.promptTokens = e.prompt_tokens;
      usage.completionTokens = e.completion_tokens;
    } else if (e.type === "error") failure = e.message;
  };
  const onAbort = () => void commands.chatCancel();
  signal.addEventListener("abort", onAbort);
  try {
    const res = await commands.chatStream(fitHistory(history, llm.contextTokens), channel);
    if (res.status === "error") throw new Error(res.error);
    if (failure) throw new Error(failure);
    if (signal.aborted) throw new DOMException("Stopped", "AbortError");
    return usage;
  } finally {
    signal.removeEventListener("abort", onAbort);
  }
};

/** Mirrors `Tuning::default()` in llm/engine.rs. */
const DEFAULT_TUNING: Tuning = { gpu_layers: null, flash_attn: true, kv_q8: false, context: null, threads: null, batch: null, experts_on_cpu: null, expert_cache_mib: null };

class LlmStore {
  engine = $state<EngineState>({ state: "stopped" });
  models = $state<ModelStatus[]>([]);
  downloads = $state<Record<string, DownloadProgress>>({});
  engines = $state<EngineStatus[]>([]);
  paths = $state<StoragePaths | null>(null);
  error = $state<string | null>(null);
  /** Context window of the loaded model, from the engine itself. */
  get contextTokens() {
    return this.engine.state === "ready" ? this.engine.context : DEFAULT_CONTEXT_TOKENS;
  }

  /** Models fully on disk, ready to load. */
  get downloaded() {
    return this.models.filter((m) => m.file.state === "downloaded");
  }

  /** The model that is loaded or loading, if any. */
  get current() {
    const e = this.engine;
    return e.state === "ready" || e.state === "loading"
      ? this.models.find((m) => m.id === e.model)
      : undefined;
  }

  get canSeeImages() {
    return this.engine.state === "ready" && (this.current?.vision ?? false);
  }

  /** Loads a model after checking it fits in memory; the failure shows in `error`. */
  async switchTo(id: string) {
    if (this.current?.id === id) return;
    // Decision (product): chat queues behind a running image instead of fighting it for memory.
    await images.idle();
    const check = await this.memory(id);
    if (check && !check.fits) {
      const gb = (b: number) => (b / 1_073_741_824).toFixed(1);
      this.error = `Needs ${gb(check.required_bytes)} GB, ${gb(check.free_bytes)} GB free. Close other apps or pick a smaller model.`;
      return;
    }
    await this.start(id);
  }

  /** True once the first catalog and engine lists have arrived. */
  loaded = $state(false);
  /** Progress of the one-button first-run setup, or null when it is not running. */
  setup = $state<SetupState | null>(null);
  private lastModelId: string | null = null;
  private autoTried = false;
  private started = false;

  /** First run: nothing to run models with yet (no engine, or no model on disk). */
  get needsSetup() {
    return this.loaded && inTauri() && (!this.engines.some((e) => e.installed) || this.downloaded.length === 0);
  }

  /** The engine and model first-run setup would fetch: the recommended ones. */
  get setupPlan() {
    const engine = this.engines.find((e) => e.recommended) ?? this.engines[0];
    const model = this.models.find((m) => m.recommended) ?? this.models[0];
    return engine && model ? { engine, model } : null;
  }

  async init() {
    if (this.started) return;
    if (!inTauri()) {
      this.loaded = true;
      return;
    }
    this.started = true;
    await events.llmStateChanged.listen((e) => this.setEngine(e.payload));
    const status = await commands.llmStatus();
    if (status.status === "ok") this.setEngine(status.data);
    await Promise.all([this.refresh(), this.refreshEngines(), this.refreshEnginePref(), this.refreshTuning(), this.refreshPaths(), chat.init()]);
    const last = await commands.llmLastModel();
    this.lastModelId = last ?? null;
    chat.needModel = () => void this.ensureLoaded();
    this.loaded = true;
    // Decision (architecture): nothing loads at launch. Chat and dictation share one small GPU,
    // so the chat model loads on the first message instead (see ensureLoaded and Residency).
  }

  /** The model to load without asking: the last used, else the recommended, else any on disk. */
  private autoTarget() {
    return pickAutoModel(this.downloaded, this.lastModelId);
  }

  /** Loads a model if none is loaded and everything needed is already on disk. */
  async ensureLoaded() {
    if (this.engine.state === "ready" || this.engine.state === "loading") return;
    if (!this.engines.some((e) => e.installed)) return;
    const target = this.autoTarget();
    if (!target) return;
    await this.switchTo(target.id);
  }

  /**
   * First-run setup in one go: install the recommended engine, download the recommended model,
   * load it. Each step is skipped if it is already done, so retrying resumes where it stopped.
   */
  async runSetup() {
    const plan = this.setupPlan;
    if (!plan) {
      this.setup = { step: "failed", message: "Airlock has no engine or model for this system." };
      return;
    }
    this.error = null;
    const fail = (message: string) => {
      this.setup = { step: "failed", message };
    };
    this.setup = { step: "engine" };
    if (!plan.engine.installed) {
      await this.installEngine(plan.engine.backend);
      if (!this.engines.find((e) => e.backend === plan.engine.backend)?.installed) {
        return fail(this.error ?? "The engine could not be installed.");
      }
    }
    this.setup = { step: "model" };
    if (plan.model.file.state !== "downloaded") {
      await this.download(plan.model.id);
      if (this.models.find((m) => m.id === plan.model.id)?.file.state !== "downloaded") {
        return fail(this.error ?? "The model could not be downloaded.");
      }
    }
    this.setup = { step: "loading" };
    await this.switchTo(plan.model.id);
    if (this.error) return fail(this.error);
    this.setup = { step: "done" };
    // Leave the finished steps visible for a moment before the card goes away.
    setTimeout(() => (this.setup = null), 1500);
  }

  private setEngine(state: EngineState) {
    this.engine = state;
    const model = state.state === "ready" || state.state === "loading" ? state.model : null;
    chat.attach(state.state === "ready" ? transport : null, toChatEngine(state), model);
  }

  async refresh() {
    const res = await commands.llmCatalog();
    if (res.status === "ok") this.models = res.data;
    else this.error = res.error;
  }

  /** Engine used for the next model load; "auto" picks CUDA, then Vulkan, then CPU. */
  enginePref = $state<Backend>("auto");

  tuning = $state<Tuning>({ ...DEFAULT_TUNING });

  async refreshTuning() {
    this.tuning = await commands.llmGetTuning();
  }

  async setTuning(patch: Partial<Tuning>) {
    const next = { ...this.tuning, ...patch };
    const res = await commands.llmSetTuning(next);
    if (res.status === "ok") this.tuning = next;
    else this.error = res.error;
  }

  resetTuning() {
    return this.setTuning({ ...DEFAULT_TUNING });
  }

  async refreshEnginePref() {
    this.enginePref = await commands.llmGetEngine();
  }

  async setEnginePref(backend: Backend) {
    const res = await commands.llmSetEngine(backend);
    if (res.status === "ok") this.enginePref = backend;
    else this.error = res.error;
  }

  async refreshEngines() {
    const res = await commands.llmEngines();
    if (res.status === "ok") this.engines = res.data;
    else this.error = res.error;
  }

  async refreshPaths() {
    const res = await commands.llmStoragePaths();
    if (res.status === "ok") this.paths = res.data;
  }

  /** `null` resets that folder to its default. Existing files are not moved. */
  async setFolder(kind: "models" | "engines", path: string | null) {
    const res = await commands.llmSetStoragePaths(
      kind === "models" ? path : null,
      kind === "engines" ? path : null,
      kind === "models",
      kind === "engines",
    );
    if (res.status === "error") {
      this.error = res.error;
      return;
    }
    this.error = null;
    this.paths = res.data;
    await Promise.all([this.refresh(), this.refreshEngines()]);
  }

  async installEngine(backend: Backend) {
    this.error = null;
    const key = `engine:${backend}`;
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
    const res = await commands.llmInstallEngine(backend, channel);
    if (res.status === "error") this.error = res.error;
    delete this.downloads[key];
    await this.refreshEngines();
  }

  cancelEngineInstall(backend: Backend) {
    return commands.llmDownloadCancel(`engine:llama-${backend}`);
  }

  async removeEngine(backend: Backend) {
    const res = await commands.llmRemoveEngine(backend);
    if (res.status === "error") this.error = res.error;
    await this.refreshEngines();
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

  /** Latest speed test of the loaded chat model, for the in-memory popover. */
  speed = $state<{ model: string; value: number } | null>(null);
  testingSpeed = $state(false);

  async testSpeed() {
    const run = this.speedRunner;
    const model = this.current?.id;
    if (!run || !model || this.testingSpeed) return;
    this.testingSpeed = true;
    try {
      const [r] = await run("chat");
      this.speed = { model, value: r.value };
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.testingSpeed = false;
    }
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
