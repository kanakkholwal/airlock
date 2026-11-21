<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import { CpuIcon, DangerTriangleIcon, EyeClosedIcon, FolderIcon, InfoCircleIcon, MicrophoneIcon, MonitorIcon, MoonIcon, PaletteIcon, PowerIcon, SettingsIcon, ShieldCheckIcon, Sun2Icon, WidgetIcon } from "@solar-icons/svelte/linear";
  import { commands, type Theme } from "../../bindings";
  import { APP_URL } from "#lib/app-info.js";
  import { inTauri } from "#lib/env.js";
  import { general } from "#lib/general-settings.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import { memory } from "#lib/memory-settings.svelte.js";
  import type { ModelUnloadTimeout } from "../../bindings";
  import type { Accelerator } from "#lib/memory-policy.js";
  import { withThemeReveal } from "#lib/theme.js";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import AssistantSettings from "#lib/components/app/assistant-settings.svelte";
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import SettingsGroup from "#lib/components/app/settings-group.svelte";
  import SettingsSkeleton from "#lib/components/app/settings-skeleton.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";

  let version = $state("");
  let tab = $state("general");

  onMount(() => {
    void general.load();
    void llm.init();
    void memory.load();
    if (inTauri()) void getVersion().then((v) => (version = v));
  });

  async function chooseFolder(kind: "models" | "engines") {
    const dir = await pickFolder({ directory: true, multiple: false });
    if (typeof dir === "string") await llm.setFolder(kind, dir);
  }

  const sections = [
    { id: "general", label: "General", icon: SettingsIcon },
    { id: "assistant", label: "Assistant", icon: ShieldCheckIcon },
    { id: "privacy", label: "Privacy", icon: ShieldCheckIcon },
    { id: "storage", label: "Storage", icon: FolderIcon },
  ];

  // Decision (product): this list is the product. Everything that can leave the computer is
  // named here, with when and where to, so "private" is something you can check.
  const boundary = $derived([
    { title: "Model search", detail: "Hugging Face, when you search. Your words are sent.", badge: "On request" },
    { title: "Model downloads", detail: "Hugging Face, when you press Download.", badge: "On request" },
    { title: "Engine downloads", detail: "GitHub, when you press Install.", badge: "On request" },
    { title: "Chats, images and voice", detail: "Stays on this computer.", badge: "Never leaves" },
    {
      title: "Update checks",
      detail: "GitHub. Version number only.",
      badge: general.updateChecks && !general.updateChecksLocked ? "On" : "Off",
    },
  ]);

  const idleOptions = [
    { value: "0", label: "Keep loaded" },
    { value: "5", label: "After 5 minutes idle" },
    { value: "15", label: "After 15 minutes idle" },
    { value: "30", label: "After 30 minutes idle" },
  ];
  const speechUnloadOptions: { value: ModelUnloadTimeout; label: string }[] = [
    { value: "never", label: "Keep loaded" },
    { value: "sec_15", label: "After 15 seconds" },
    { value: "min_2", label: "After 2 minutes" },
    { value: "min_5", label: "After 5 minutes" },
    { value: "min_10", label: "After 10 minutes" },
    { value: "min_15", label: "After 15 minutes" },
    { value: "hour_1", label: "After 1 hour" },
    { value: "immediately", label: "Right after each dictation" },
  ];
  const acceleratorOptions: { value: Accelerator; label: string }[] = [
    { value: "auto", label: "Automatic" },
    { value: "cpu", label: "CPU (keeps the GPU free for chat)" },
    { value: "gpu", label: "GPU" },
  ];

  const folders = $derived(
    llm.paths
      ? [
          { kind: "models" as const, title: "Models folder", path: llm.paths.models_dir, custom: llm.paths.models_custom },
          { kind: "engines" as const, title: "Engines folder", path: llm.paths.engines_dir, custom: llm.paths.engines_custom },
        ]
      : [],
  );
</script>

<PageScroll>
  <main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-12">
    <PageHeader title="Settings" />

    {#if !inTauri()}
      <Alert variant="info"><InfoCircleIcon size={18} />
        <AlertDescription>
          Browser preview. Settings are not saved.
        </AlertDescription>
      </Alert>
    {/if}

    {#if general.error}
      <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{general.error}</AlertDescription></Alert>
    {/if}

    {#if !general.loaded}
      <SettingsSkeleton groups={2} rows={3} />
    {:else}
      <Tabs bind:value={tab} variant="segment" class="self-center">
        <TabsList>
          {#each sections as s (s.id)}
            <TabsTrigger value={s.id}>{s.label}</TabsTrigger>
          {/each}
        </TabsList>
      </Tabs>

          {#if tab === "general"}
<SettingsGroup id="general" icon={SettingsIcon} title="General">
            <SettingRow icon={PaletteIcon} title="Theme">
              <ToggleGroup
                label="Theme"
                value={general.theme}
                onValueChange={(v) => {
                  if (typeof v === "string" && v) withThemeReveal(() => void general.setTheme(v as Theme));
                }}
              >
                <ToggleGroupItem value="system" aria-label="System"><MonitorIcon size={16} /></ToggleGroupItem>
                <ToggleGroupItem value="light" aria-label="Light"><Sun2Icon size={16} /></ToggleGroupItem>
                <ToggleGroupItem value="dark" aria-label="Dark"><MoonIcon size={16} /></ToggleGroupItem>
              </ToggleGroup>
            </SettingRow>
            <SettingRow icon={PowerIcon} title="Start at login" description="So dictation is always ready.">
              <Switch aria-label="Start at login" checked={general.autostart} onCheckedChange={(v) => general.setAutostart(v)} />
            </SettingRow>
            <SettingRow icon={EyeClosedIcon} title="Start hidden">
              <Switch aria-label="Start hidden" checked={general.startHidden} onCheckedChange={(v) => general.setStartHidden(v)} />
            </SettingRow>
            <SettingRow icon={WidgetIcon} title="Tray icon">
              <Switch aria-label="Tray icon" checked={general.showTray} onCheckedChange={(v) => general.setShowTray(v)} />
            </SettingRow>
          </SettingsGroup>

          {/if}
{#if tab === "general"}
<SettingsGroup
  id="memory"
  icon={CpuIcon}
  title="Memory" description="Shared by chat and dictation."
>
  <SettingRow icon={CpuIcon} title="Free chat model memory" description="It reloads on your next message.">
    <Select
      items={idleOptions}
      bind:value={() => String(memory.idleMinutes), (v) => void memory.setResidency(Number(v), memory.unloadWhenHidden)}
    >
      <SelectTrigger class="w-52">
        <span class="truncate">{idleOptions.find((o) => o.value === String(memory.idleMinutes))?.label ?? `After ${memory.idleMinutes} minutes idle`}</span>
      </SelectTrigger>
      <SelectContent align="end">
        {#each idleOptions as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
      </SelectContent>
    </Select>
  </SettingRow>
  <SettingRow icon={CpuIcon} title="Unload when the window is hidden" description="After 30 seconds hidden.">
    <Switch
      aria-label="Unload when the window is hidden"
      checked={memory.unloadWhenHidden}
      onCheckedChange={(v) => memory.setResidency(memory.idleMinutes, v)}
    />
  </SettingRow>
  <SettingRow icon={MicrophoneIcon} title="Free speech model memory" description="It reloads on your next dictation.">
    <Select
      items={speechUnloadOptions}
      bind:value={() => memory.speechUnload, (v) => void memory.setSpeechUnload(v as ModelUnloadTimeout)}
    >
      <SelectTrigger class="w-52">
        <span class="truncate">{speechUnloadOptions.find((o) => o.value === memory.speechUnload)?.label}</span>
      </SelectTrigger>
      <SelectContent align="end">
        {#each speechUnloadOptions as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
      </SelectContent>
    </Select>
  </SettingRow>
  <SettingRow
    title="Speech processing"
    description={memory.gpuNames.length > 0
      ? `Where dictation runs. A short phrase takes about a second on the CPU. Your GPU: ${memory.gpuNames.join(", ")}.`
      : "Where dictation runs. A short phrase takes about a second on the CPU."}
  >
    <Select
      items={acceleratorOptions}
      bind:value={() => memory.accelerator, (v) => void memory.setAccelerator(v as Accelerator)}
    >
      <SelectTrigger class="w-52">
        <span class="truncate">{acceleratorOptions.find((o) => o.value === memory.accelerator)?.label}</span>
      </SelectTrigger>
      <SelectContent align="end">
        {#each acceleratorOptions as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
      </SelectContent>
    </Select>
  </SettingRow>
  <p class="pb-3 text-sm text-muted-foreground text-pretty">
    Cleanup needs the chat model loaded.
  </p>
</SettingsGroup>
{/if}
{#if tab === "assistant"}
<AssistantSettings />
{/if}
{#if tab === "privacy"}
<SettingsGroup id="privacy" icon={ShieldCheckIcon} title="Privacy">
            {#each boundary as item (item.title)}
              <SettingRow title={item.title} description={item.detail}>
                <Badge variant={item.badge === "Never leaves" ? "success" : "secondary"}>{item.badge}</Badge>
              </SettingRow>
            {/each}
            <SettingRow
              title="Check for updates"
              description={general.updateChecksLocked ? "Turned off by system configuration." : "Look for new versions of Airlock."}
            >
              <Switch
                aria-label="Check for updates"
                checked={general.updateChecks && !general.updateChecksLocked}
                disabled={general.updateChecksLocked}
                onCheckedChange={(v) => general.setUpdateChecks(v)}
              />
            </SettingRow>
          </SettingsGroup>

          {/if}
{#if tab === "storage"}
<SettingsGroup
            id="storage"
            icon={FolderIcon}
            title="Storage"
          >
            {#each folders as row (row.kind)}
              <SettingRow title={row.title} description={row.path}>
                <div class="flex gap-2">
                  <Button size="sm" variant="secondary" onclick={() => chooseFolder(row.kind)}>
                    <FolderIcon size={16} />Change
                  </Button>
                  {#if row.custom}
                    <Button size="sm" variant="ghost" onclick={() => llm.setFolder(row.kind, null)}>Reset</Button>
                  {/if}
                </div>
              </SettingRow>
            {:else}
              <SettingRow title="Folders">
                <span></span>
              </SettingRow>
            {/each}
            <SettingRow icon={FolderIcon} title="Open data folder" description="Chats, files and settings.">
              <Button size="sm" variant="secondary" onclick={() => commands.openAppDataDir()}>Open</Button>
            </SettingRow>
          </SettingsGroup>

          {/if}
{#if tab === "general"}
<SettingsGroup id="about" icon={InfoCircleIcon} title="About">
            <SettingRow title="Airlock">
              <Badge variant="secondary">{version ? `v${version}` : "Preview"}</Badge>
            </SettingRow>
            <SettingRow title="Website" description={APP_URL.replace("https://", "")}>
              <Button size="sm" variant="secondary" onclick={() => openUrl(APP_URL)}>Open</Button>
            </SettingRow>
          </SettingsGroup>
{/if}
    {/if}
  </main>
</PageScroll>
