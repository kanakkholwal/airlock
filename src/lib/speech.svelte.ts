import { listen } from "@tauri-apps/api/event";
import {
  commands,
  events,
  type AudioDevice,
  type HistoryEntry,
  type ModelInfo,
  type TranscriptionJobStatus,
  type WindowsMicrophonePermissionStatus,
} from "../bindings";
import { checkAccess, isMac, requestAccess, startDictationEngine, type DictationAccess } from "./dictation/access";
import { inTauri } from "./env";

export type SpeechProgress = {
  downloaded: number;
  total: number;
  percentage: number;
  bytesPerSecond: number | null;
};

type DownloadProgressEvent = {
  model_id: string;
  downloaded: number;
  total: number;
  percentage: number;
  bytes_per_sec: number | null;
};

const HISTORY_PAGE = 30;

/** Speech-to-text state: the speech model, microphone and permission, and dictation history. */
class SpeechStore {
  loaded = $state(false);
  error = $state<string | null>(null);
  models = $state<ModelInfo[]>([]);
  current = $state<string | null>(null);
  progress = $state<Record<string, SpeechProgress>>({});
  mics = $state<AudioDevice[]>([]);
  mic = $state("Default");
  permission = $state<WindowsMicrophonePermissionStatus | null>(null);
  history = $state<HistoryEntry[]>([]);
  historyLoaded = $state(false);
  hasMoreHistory = $state(false);
  /** macOS permissions; always granted elsewhere. */
  access = $state<DictationAccess>({ accessibility: true, microphone: true });
  /** True once the shortcut and paste engine are registered. */
  engineReady = $state(false);
  /** Entry ids of files queued this session, newest first. */
  fileJobs = $state<number[]>([]);
  /** Live state of each file transcription, by history entry id. */
  jobs = $state<Record<number, TranscriptionJobStatus>>({});
  /** Files that could not even be queued, such as ones with no audio. */
  fileProblems = $state<{ name: string; error: string }[]>([]);
  /** Timing of the latest dictation, so CPU and GPU can be compared by eye. */
  lastRun = $state<{ audioSeconds: number; waitSeconds: number } | null>(null);
  /** The speech model held in memory right now, and whether one is loading. */
  loadedModel = $state<string | null>(null);
  loadingModel = $state(false);
  /** True while the one-button setup runs. */
  settingUp = $state(false);
  private started = false;

  /** Handy marks a short list as recommended; the rest sit behind "More models". */
  get recommended() {
    return this.models.filter((m) => m.is_recommended);
  }

  get others() {
    return this.models.filter((m) => !m.is_recommended).sort((a, b) => a.size_mb - b.size_mb);
  }

  get downloaded() {
    return this.models.filter((m) => m.is_downloaded);
  }

  /** First run: no speech model on disk yet. */
  get needsSetup() {
    return this.loaded && inTauri() && this.downloaded.length === 0;
  }

  get setupModel() {
    return this.recommended[0] ?? this.models[0];
  }

  /** macOS still needs a permission before the shortcut can work. */
  get needsAccess() {
    return isMac() && (!this.access.accessibility || !this.access.microphone);
  }

  get micBlocked() {
    return !!this.permission?.supported && this.permission.overall_access === "denied";
  }

  async init() {
    if (this.started) return;
    if (!inTauri()) {
      this.loaded = true;
      this.historyLoaded = true;
      return;
    }
    this.started = true;
    const refresh = () => void this.refreshModels();
    await Promise.all([
      listen<DownloadProgressEvent>("model-download-progress", (e) => {
        const p = e.payload;
        this.progress[p.model_id] = {
          downloaded: p.downloaded,
          total: p.total,
          percentage: p.percentage,
          bytesPerSecond: p.bytes_per_sec,
        };
      }),
      listen("model-download-complete", refresh),
      listen("model-download-cancelled", refresh),
      listen("model-deleted", refresh),
      listen("models-updated", refresh),
      listen<{ event_type: string }>("model-state-changed", (e) => {
        this.loadingModel = e.payload.event_type === "loading_started";
        void this.refreshLoaded();
      }),
      events.historyUpdatePayload.listen((e) => this.onHistory(e.payload)),
      events.transcriptionCompleteEvent.listen((e) => {
        if (e.payload.audio_seconds > 0) {
          this.lastRun = { audioSeconds: e.payload.audio_seconds, waitSeconds: e.payload.wait_seconds };
        }
      }),
      events.transcriptionJob.listen((e) => {
        this.jobs[e.payload.entry_id] = e.payload.status;
      }),
    ]);
    await Promise.all([this.refreshModels(), this.refreshMics(), this.refreshPermission(), this.refreshLoaded()]);
    await this.adoptDownloadedModel();
    this.loaded = true;
    void this.loadHistory(true);
    await this.startEngine();
  }

  /**
   * Decision (product): a speech model already on disk is simply used. Without this, a model
   * found in a shared cache is never selected and the shortcut does nothing.
   */
  private async adoptDownloadedModel() {
    if (this.current || this.downloaded.length === 0) return;
    const pick = this.downloaded.find((m) => m.is_recommended) ?? this.downloaded[0];
    await commands.completeOnboarding();
    await this.use(pick.id);
  }

  /** Registers the shortcut and paste engine once macOS permissions (if any) are in place. */
  async startEngine() {
    this.access = await checkAccess();
    if (this.needsAccess) return;
    const problem = await startDictationEngine();
    this.engineReady = problem === null;
    if (problem) this.error = problem;
  }

  async allow(kind: keyof DictationAccess) {
    await requestAccess(kind);
    await this.startEngine();
  }

  async refreshModels() {
    const res = await commands.getAvailableModels();
    if (res.status === "ok") this.models = res.data;
    else this.error = res.error;
    const cur = await commands.getCurrentModel();
    this.current = cur.status === "ok" && cur.data ? cur.data : null;
    // Finished or cancelled downloads no longer need a progress bar.
    for (const m of this.models) if (!m.is_downloading) delete this.progress[m.id];
  }

  async refreshLoaded() {
    const res = await commands.getModelLoadStatus();
    if (res.status === "ok") this.loadedModel = res.data.is_loaded ? res.data.current_model : null;
  }

  async unload() {
    const res = await commands.unloadModelManually();
    if (res.status === "error") this.error = res.error;
    await this.refreshLoaded();
  }

  async refreshMics() {
    const [list, selected] = await Promise.all([commands.getAvailableMicrophones(), commands.getSelectedMicrophone()]);
    if (list.status === "ok") this.mics = list.data;
    if (selected.status === "ok") this.mic = selected.data;
  }

  async refreshPermission() {
    this.permission = await commands.getWindowsMicrophonePermissionStatus();
  }

  async download(id: string) {
    this.error = null;
    this.progress[id] = { downloaded: 0, total: 0, percentage: 0, bytesPerSecond: null };
    const res = await commands.downloadModel(id);
    if (res.status === "error") this.error = res.error;
    await this.refreshModels();
  }

  cancel(id: string) {
    return commands.cancelDownload(id);
  }

  async use(id: string) {
    this.error = null;
    const res = await commands.setActiveModel(id);
    if (res.status === "error") this.error = res.error;
    await this.refreshModels();
  }

  async remove(id: string) {
    const res = await commands.deleteModel(id);
    if (res.status === "error") this.error = res.error;
    await this.refreshModels();
  }

  async setMic(name: string) {
    this.mic = name;
    const res = await commands.setSelectedMicrophone(name);
    if (res.status === "error") this.error = res.error;
  }

  /** First-run setup: fetch the recommended speech model and make it the one in use. */
  async runSetup() {
    const model = this.setupModel;
    if (!model) return;
    this.settingUp = true;
    try {
      if (!model.is_downloaded) await this.download(model.id);
      if (this.models.find((m) => m.id === model.id)?.is_downloaded) {
        await commands.completeOnboarding();
        await this.use(model.id);
        await this.startEngine();
      }
    } finally {
      this.settingUp = false;
    }
  }

  async loadHistory(reset = false) {
    if (!inTauri()) return;
    const cursor = reset || this.history.length === 0 ? null : this.history[this.history.length - 1].id;
    const res = await commands.getHistoryEntries(cursor, HISTORY_PAGE);
    if (res.status === "ok") {
      this.history = reset ? res.data.entries : [...this.history, ...res.data.entries];
      this.hasMoreHistory = res.data.has_more;
    } else {
      this.error = res.error;
    }
    this.historyLoaded = true;
  }

  private onHistory(p: import("../bindings").HistoryUpdatePayload) {
    if (p.action === "added") this.history = [p.entry, ...this.history.filter((h) => h.id !== p.entry.id)];
    else if (p.action === "updated") this.history = this.history.map((h) => (h.id === p.entry.id ? p.entry : h));
    else if (p.action === "deleted") this.history = this.history.filter((h) => h.id !== p.id);
    else if (p.action === "toggled") this.history = this.history.map((h) => (h.id === p.id ? { ...h, saved: !h.saved } : h));
  }

  /** Adds audio or video files to History and queues each for transcription. */
  async transcribeFiles(paths: string[]) {
    this.fileProblems = [];
    for (const path of paths) {
      const res = await commands.transcribeFile(path);
      if (res.status === "ok") {
        this.fileJobs = [res.data.id, ...this.fileJobs.filter((id) => id !== res.data.id)];
        if (!this.history.some((h) => h.id === res.data.id)) this.history = [res.data, ...this.history];
      } else {
        this.fileProblems.push({ name: path.split(/[\/]/).pop() ?? path, error: res.error });
      }
    }
  }

  cancelJob(id: number) {
    return commands.cancelHistoryEntryTranscription(id);
  }

  retryJob(id: number) {
    return commands.retryHistoryEntryTranscription(id);
  }

  async toggleSaved(id: number) {
    const res = await commands.toggleHistoryEntrySaved(id);
    if (res.status === "error") this.error = res.error;
  }

  async removeEntry(id: number) {
    const res = await commands.deleteHistoryEntry(id);
    if (res.status === "error") this.error = res.error;
  }
}

export const speech = new SpeechStore();
