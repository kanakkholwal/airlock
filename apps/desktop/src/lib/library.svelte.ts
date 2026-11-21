import { commands, events, type Artifact, type Job, type Privacy } from "../bindings";
import { inTauri } from "./env";
import { llm } from "./llm.svelte";

export type Kind = "" | "recording" | "transcript" | "image" | "note" | "summary";

/** Kinds that have text a model can read or summarize. */
export const hasText = (kind: string) => kind === "recording" || kind === "transcript" || kind === "note" || kind === "summary";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Everything Airlock has made or kept, with its privacy flag, and the jobs that work on it. */
class LibraryStore {
  items = $state<Artifact[]>([]);
  jobs = $state<Job[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);
  kind = $state<Kind>("");
  query = $state("");
  private started = false;

  async init() {
    if (!inTauri()) {
      this.loaded = true;
      return;
    }
    if (!this.started) {
      this.started = true;
      await events.jobUpdated.listen((e) => {
        const job: Job = e.payload;
        this.jobs = [job, ...this.jobs.filter((j) => j.id !== job.id)];
        if (job.state === "done") void this.load();
      });
    }
    await commands.libraryRefresh();
    await Promise.all([this.load(), this.loadJobs()]);
    this.loaded = true;
  }

  async load() {
    const res = await commands.libraryList(this.kind || null, this.query.trim() || null);
    if (res.status === "ok") this.items = res.data;
    else this.error = res.error;
  }

  async loadJobs() {
    const res = await commands.jobList();
    if (res.status === "ok") this.jobs = res.data;
  }

  setKind(kind: Kind) {
    this.kind = kind;
    return this.load();
  }

  setQuery(query: string) {
    this.query = query;
    return this.load();
  }

  async setPrivacy(id: string, privacy: Privacy) {
    const res = await commands.librarySetPrivacy(id, privacy);
    if (res.status === "error") this.error = res.error;
    else this.items = this.items.map((a) => (a.id === id ? { ...a, privacy } : a));
  }

  /** The job working on this item right now, if any. */
  jobFor(id: string) {
    return this.jobs.find((j) => j.input_id === id && (j.state === "queued" || j.state === "running"));
  }

  /** The newest finished or stopped job for this item, so a failure can be shown and retried. */
  lastJobFor(id: string) {
    return this.jobs.find((j) => j.input_id === id && j.state !== "queued" && j.state !== "running" && j.state !== "done");
  }

  /** Loads the chat model if it is not loaded, so the job does not fail on a missing model. */
  private async ensureModel() {
    const state = () => llm.engine.state as string;
    if (state() === "ready") return true;
    await llm.ensureLoaded();
    for (let i = 0; i < 120; i++) {
      if (state() === "ready") return true;
      if (state() === "failed") return false;
      await sleep(500);
    }
    return false;
  }

  async summarize(id: string) {
    this.error = null;
    if (!(await this.ensureModel())) {
      this.error = "Load a chat model first, then try again.";
      return;
    }
    const res = await commands.jobStart("summarize", id);
    if (res.status === "error") this.error = res.error;
    else this.jobs = [res.data, ...this.jobs.filter((j) => j.id !== res.data.id)];
  }

  async retry(id: number) {
    if (!(await this.ensureModel())) {
      this.error = "Load a chat model first, then try again.";
      return;
    }
    const res = await commands.jobRetry(id);
    if (res.status === "error") this.error = res.error;
  }

  cancel(id: number) {
    return commands.jobCancel(id);
  }

  async clearJob(id: number) {
    await commands.jobClear(id);
    this.jobs = this.jobs.filter((j) => j.id !== id);
  }

  async text(id: string): Promise<string | null> {
    const res = await commands.libraryText(id);
    if (res.status === "ok") return res.data;
    this.error = res.error;
    return null;
  }
}

export const library = new LibraryStore();
