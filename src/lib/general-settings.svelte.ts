import { commands, type Theme } from "../bindings";
import { inTauri } from "./env";
import { applyTheme } from "./theme";

/** App-wide preferences (look, startup, updates). Persisted by the backend settings store. */
class GeneralSettings {
  loaded = $state(false);
  error = $state<string | null>(null);
  theme = $state<Theme>("system");
  autostart = $state(false);
  startHidden = $state(false);
  showTray = $state(true);
  updateChecks = $state(true);
  updateChecksLocked = $state(false);

  async load() {
    if (!inTauri()) {
      this.loaded = true;
      return;
    }
    const [res, locked] = await Promise.all([commands.getAppSettings(), commands.isUpdateChecksLocked()]);
    this.updateChecksLocked = locked;
    if (res.status === "error") {
      this.error = res.error;
      return;
    }
    const s = res.data;
    this.theme = s.theme ?? "system";
    this.autostart = s.autostart_enabled ?? false;
    this.startHidden = s.start_hidden ?? false;
    this.showTray = s.show_tray_icon ?? true;
    this.updateChecks = s.update_checks_enabled ?? true;
    this.loaded = true;
  }

  private async apply(call: () => Promise<{ status: "ok" | "error"; error?: string }>) {
    if (!inTauri()) return;
    const res = await call();
    this.error = res.status === "error" ? (res.error ?? "Failed") : null;
  }

  setTheme(value: Theme) {
    this.theme = value;
    applyTheme(value);
    return this.apply(() => commands.changeThemeSetting(value));
  }
  setAutostart(value: boolean) {
    this.autostart = value;
    return this.apply(() => commands.changeAutostartSetting(value));
  }
  setStartHidden(value: boolean) {
    this.startHidden = value;
    return this.apply(() => commands.changeStartHiddenSetting(value));
  }
  setShowTray(value: boolean) {
    this.showTray = value;
    return this.apply(() => commands.changeShowTrayIconSetting(value));
  }
  setUpdateChecks(value: boolean) {
    this.updateChecks = value;
    return this.apply(() => commands.changeUpdateChecksSetting(value));
  }
}

export const general = new GeneralSettings();
