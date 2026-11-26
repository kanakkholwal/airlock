import { commands, events, type Automation, type AutomationInput, type Run, type TimezoneInfo } from "../bindings";
import { inTauri } from "./env";

/** Scheduled work that runs on this computer. Rust owns the schedule; this only shows and edits it. */
class AutomationStore {
  items = $state<Automation[]>([]);
  runs = $state<Record<number, Run[]>>({});
  /** Ids started with "Run now" that have not reported back yet. */
  running = $state<number[]>([]);
  zone = $state<TimezoneInfo | null>(null);
  loaded = $state(false);
  error = $state<string | null>(null);
  private started = false;

  async init() {
    if (!inTauri()) {
      this.loaded = true;
      return;
    }
    if (!this.started) {
      this.started = true;
      await events.automationUpdated.listen((e) => {
        const a: Automation = e.payload;
        this.items = this.items.some((x) => x.id === a.id) ? this.items.map((x) => (x.id === a.id ? a : x)) : [...this.items, a];
        this.running = this.running.filter((id) => id !== a.id);
        void this.loadRuns(a.id);
      });
    }
    this.zone = await commands.automationTimezone();
    await this.load();
    this.loaded = true;
  }

  async load() {
    const res = await commands.automationList();
    if (res.status === "error") {
      this.error = res.error;
      return;
    }
    this.items = res.data;
    await Promise.all(res.data.map((a) => this.loadRuns(a.id)));
  }

  async loadRuns(id: number) {
    const res = await commands.automationRuns(id);
    if (res.status === "ok") this.runs = { ...this.runs, [id]: res.data };
  }

  lastRun(id: number): Run | undefined {
    return this.runs[id]?.[0];
  }

  async save(input: AutomationInput): Promise<string | null> {
    const res = await commands.automationSave(input);
    if (res.status === "error") return res.error;
    this.items = this.items.some((x) => x.id === res.data.id) ? this.items.map((x) => (x.id === res.data.id ? res.data : x)) : [...this.items, res.data];
    return null;
  }

  async setEnabled(a: Automation, enabled: boolean) {
    const err = await this.save({ id: a.id, name: a.name, cron: a.cron, action: a.action, enabled, catchup: a.catchup });
    if (err) this.error = err;
  }

  async remove(id: number) {
    const res = await commands.automationDelete(id);
    if (res.status === "error") {
      this.error = res.error;
      return;
    }
    this.items = this.items.filter((a) => a.id !== id);
  }

  runNow(id: number) {
    this.running = [...this.running, id];
    void commands.automationRunNow(id);
  }

  async setZone(setting: string): Promise<string | null> {
    const res = await commands.automationSetTimezone(setting);
    if (res.status === "error") return res.error;
    this.zone = res.data;
    await this.load();
    return null;
  }
}

export const automations = new AutomationStore();

export const ACTIONS: { value: string; label: string; hint: string }[] = [
  { value: "summarize_new", label: "Summarize new notes and recordings", hint: "Uses your local chat model. Nothing leaves this computer." },
];

export const actionLabel = (value: string) => ACTIONS.find((a) => a.value === value)?.label ?? value;
