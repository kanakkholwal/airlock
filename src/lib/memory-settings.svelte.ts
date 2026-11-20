import { commands, type ModelUnloadTimeout } from "../bindings";
import { inTauri } from "./env";
import { defaultAccelerator, type Accelerator } from "./memory-policy";

const APPLIED = "airlock-accelerator-default-applied";

/** How much memory Airlock keeps in use: when the chat model unloads, and where speech runs. */
class MemorySettings {
  loaded = $state(false);
  error = $state<string | null>(null);
  idleMinutes = $state(5);
  unloadWhenHidden = $state(true);
  accelerator = $state<Accelerator>("auto");
  speechUnload = $state<ModelUnloadTimeout>("min_5");
  gpuNames = $state<string[]>([]);

  async load() {
    if (!inTauri()) {
      this.loaded = true;
      return;
    }
    const [res, residency, accel] = await Promise.all([
      commands.getAppSettings(),
      commands.llmGetResidency(),
      commands.getAvailableAccelerators(),
    ]);
    this.idleMinutes = residency.idle_minutes;
    this.unloadWhenHidden = residency.unload_when_hidden;
    this.gpuNames = accel.gpu_devices.map((g) => `${g.name} (${Math.round(g.total_vram_mb / 1024)} GB)`);
    if (res.status === "ok") {
      this.accelerator = res.data.transcribe_accelerator ?? "auto";
      this.speechUnload = res.data.model_unload_timeout ?? "min_5";
    }

    // One time: choose the better starting point for this machine. After that it is the user's.
    if (!localStorage.getItem(APPLIED)) {
      localStorage.setItem(APPLIED, "1");
      const pick = defaultAccelerator(accel.gpu_devices);
      if (pick !== this.accelerator) await this.setAccelerator(pick);
    }
    this.loaded = true;
  }

  async setResidency(idleMinutes: number, unloadWhenHidden: boolean) {
    this.idleMinutes = idleMinutes;
    this.unloadWhenHidden = unloadWhenHidden;
    const res = await commands.llmSetResidency(idleMinutes, unloadWhenHidden);
    this.error = res.status === "error" ? res.error : null;
  }

  async setSpeechUnload(value: ModelUnloadTimeout) {
    this.speechUnload = value;
    await commands.setModelUnloadTimeout(value);
    this.error = null;
  }

  async setAccelerator(value: Accelerator) {
    this.accelerator = value;
    const res = await commands.changeTranscribeAcceleratorSetting(value);
    this.error = res.status === "error" ? res.error : null;
  }
}

export const memory = new MemorySettings();
