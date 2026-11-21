<script lang="ts">
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { DangerTriangleIcon, FolderIcon, LockKeyholeIcon, TrashBin2Icon, WidgetIcon } from "@solar-icons/svelte/linear";
  import type { Mode, Risk } from "../../../bindings";
  import { agent } from "#lib/agent.svelte.js";
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import SettingsGroup from "#lib/components/app/settings-group.svelte";
  import SettingsSkeleton from "#lib/components/app/settings-skeleton.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";

  // Decision (product): the assistant starts with no folder and asks before anything private
  // or anything that saves. There is no "approve everything" switch on purpose.
  const modes: { value: Mode; label: string; hint: string }[] = [
    { value: "ask", label: "Ask me", hint: "Private reads and saves ask first." },
    { value: "accept_writes", label: "Ask, but allow saving notes", hint: "Notes save without asking. Private reads still ask." },
    { value: "dont_ask", label: "Never ask", hint: "Only tools you have allowed run. Everything else is refused." },
  ];
  const riskLabel: Record<Risk, { text: string; variant: "success" | "warning" | "info" }> = {
    read: { text: "Reads", variant: "success" },
    ask: { text: "Asks first", variant: "warning" },
    write: { text: "Saves", variant: "info" },
  };

  onMount(() => void agent.load());

  const config = $derived(agent.config);
  const current = $derived(modes.find((m) => m.value === config?.mode));

  async function addFolder() {
    const dir = await pickFolder({ directory: true, multiple: false });
    if (typeof dir === "string") await agent.addFolder(dir);
  }
</script>

{#if !config}
  <SettingsSkeleton groups={2} rows={3} />
{:else}
  {#if agent.error}
    <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{agent.error}</AlertDescription></Alert>
  {/if}

  <SettingsGroup
    id="assistant"
    icon={LockKeyholeIcon}
    title="Assistant"
  >
    <SettingRow icon={WidgetIcon} title="Let the assistant use tools">
      <Switch aria-label="Let the assistant use tools" checked={config.enabled} onCheckedChange={(v) => agent.setEnabled(v)} />
    </SettingRow>
    <SettingRow title="When to ask" description={current?.hint}>
      <Select items={modes} bind:value={() => config.mode, (v) => void agent.setMode(v as Mode)}>
        <SelectTrigger class="w-64"><span class="truncate">{current?.label}</span></SelectTrigger>
        <SelectContent align="end">
          {#each modes as m (m.value)}<SelectItem value={m.value}>{m.label}</SelectItem>{/each}
        </SelectContent>
      </Select>
    </SettingRow>
  </SettingsGroup>

  <SettingsGroup
    id="assistant-folders"
    icon={FolderIcon}
    title="Folders it can read" description="Files it can open."
  >
    {#each config.folders as folder (folder)}
      <SettingRow title={folder.split(/[\\/]/).pop() || folder} description={folder}>
        <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => agent.removeFolder(folder)}>
          <TrashBin2Icon size={16} />Remove
        </Button>
      </SettingRow>
    {:else}
      <SettingRow title="No folders shared" description="Add one to let it read your files.">
        <span></span>
      </SettingRow>
    {/each}
    <SettingRow icon={FolderIcon} title="Share a folder">
      <Button size="sm" variant="secondary" class="gap-1.5" onclick={addFolder}><FolderIcon size={16} />Add folder</Button>
    </SettingRow>
  </SettingsGroup>

  <SettingsGroup id="assistant-tools" title="Tools">
    {#each agent.tools as tool (tool.name)}
      <SettingRow title={tool.name.replaceAll("_", " ")} description={tool.description}>
        <div class="flex items-center gap-3">
          <Badge size="sm" variant={riskLabel[tool.risk].variant}>{riskLabel[tool.risk].text}</Badge>
          <Switch
            aria-label={`Allow ${tool.name}`}
            checked={tool.enabled}
            disabled={!config.enabled}
            onCheckedChange={(v) => agent.setToolEnabled(tool.name, v)}
          />
        </div>
      </SettingRow>
    {/each}
  </SettingsGroup>

  {#if (config.grants?.length ?? 0) > 0}
    <SettingsGroup
      id="assistant-grants"
      title="Always allowed" description="Remove one to be asked again."
    >
      {#each config.grants ?? [] as grant (grant.tool + (grant.pattern ?? ""))}
        <SettingRow title={grant.tool.replaceAll("_", " ")} description={grant.pattern ? `Only for ${grant.pattern}` : "Every time"}>
          <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => agent.removeGrant(grant)}>
            <TrashBin2Icon size={16} />Remove
          </Button>
        </SettingRow>
      {/each}
    </SettingsGroup>
  {/if}
{/if}
