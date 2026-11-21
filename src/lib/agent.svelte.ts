import { commands, type AgentConfig, type Mode, type Rule, type ToolInfo } from "../bindings";
import { inTauri } from "./env";

/** What the assistant may do: its tools, the folders it can read, and when it must ask. */
class AgentStore {
  config = $state<AgentConfig | null>(null);
  tools = $state<ToolInfo[]>([]);
  error = $state<string | null>(null);

  get loaded() {
    return this.config !== null;
  }

  async load() {
    if (!inTauri()) return;
    const [config, tools] = await Promise.all([commands.agentGetConfig(), commands.agentTools()]);
    this.config = config;
    this.tools = tools;
  }

  private async save(next: AgentConfig) {
    const res = await commands.agentSetConfig(next);
    if (res.status === "ok") {
      this.config = res.data;
      this.error = null;
      this.tools = await commands.agentTools();
    } else {
      this.error = res.error;
    }
  }

  setEnabled(enabled: boolean) {
    if (this.config) return this.save({ ...this.config, enabled });
  }

  setMode(mode: Mode) {
    if (this.config) return this.save({ ...this.config, mode });
  }

  setToolEnabled(name: string, on: boolean) {
    if (!this.config) return;
    const rest = this.config.disabled_tools.filter((t) => t !== name);
    return this.save({ ...this.config, disabled_tools: on ? rest : [...rest, name] });
  }

  addFolder(path: string) {
    if (this.config) return this.save({ ...this.config, folders: [...this.config.folders, path] });
  }

  removeFolder(path: string) {
    if (this.config) return this.save({ ...this.config, folders: this.config.folders.filter((f) => f !== path) });
  }

  removeGrant(rule: Rule) {
    if (this.config) return this.save({ ...this.config, grants: (this.config.grants ?? []).filter((g) => g !== rule) });
  }
}

export const agent = new AgentStore();
