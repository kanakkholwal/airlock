<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { open as pickFolder } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import {
    CloudCrossIcon,
    DownloadMinimalisticIcon,
    FolderIcon,
    MonitorIcon,
    MoonIcon,
    ShieldCheckIcon,
    Sun2Icon,
  } from "@solar-icons/svelte/linear";
  import { commands, type Theme } from "../../bindings";
  import { APP_URL } from "#lib/app-info.js";
  import { inTauri } from "#lib/env.js";
  import { general } from "#lib/general-settings.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import SettingsGroup from "#lib/components/app/settings-group.svelte";
  import SettingsSkeleton from "#lib/components/app/settings-skeleton.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";

  let version = $state("");

  onMount(() => {
    void general.load();
    void llm.init();
    if (inTauri()) void getVersion().then((v) => (version = v));
  });

  async function chooseFolder(kind: "models" | "engines") {
    const dir = await pickFolder({ directory: true, multiple: false });
    if (typeof dir === "string") await llm.setFolder(kind, dir);
  }

  // Decision (product): this list is the product. Everything that can leave the computer is
  // named here, with when and where to, so "private" is something you can check.
  const boundary = $derived([
    {
      icon: DownloadMinimalisticIcon,
      title: "Model downloads",
      detail: "huggingface.co, only when you press Download.",
      badge: "On request",
    },
    {
      icon: DownloadMinimalisticIcon,
      title: "Engine downloads",
      detail: "github.com, only when you press Install.",
      badge: "On request",
    },
    {
      icon: CloudCrossIcon,
      title: "Chats, images and voice",
      detail: "Processed on this computer. Never sent anywhere.",
      badge: "Never leaves",
    },
    {
      icon: ShieldCheckIcon,
      title: "Update checks",
      detail: "github.com, version number only.",
      badge: general.updateChecks && !general.updateChecksLocked ? "On" : "Off",
    },
  ]);

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
    <PageHeader title="Settings" description="How Airlock looks, starts, and what it is allowed to reach." />

    {#if !inTauri()}
      <Alert variant="info">
        <AlertDescription>
          You are viewing a browser preview, so these settings are not saved. Open the desktop app to change them.
        </AlertDescription>
      </Alert>
    {/if}

    {#if general.error}
      <Alert variant="destructive"><AlertDescription>{general.error}</AlertDescription></Alert>
    {/if}

    {#if !general.loaded}
      <SettingsSkeleton groups={4} rows={2} />
    {:else}
      <SettingsGroup title="Appearance">
        <SettingRow title="Theme" description="Follow your system, or choose one.">
          <ToggleGroup
            label="Theme"
            value={general.theme}
            onValueChange={(v) => {
              if (typeof v === "string" && v) void general.setTheme(v as Theme);
            }}
          >
            <ToggleGroupItem value="system" aria-label="System"><MonitorIcon size={16} /></ToggleGroupItem>
            <ToggleGroupItem value="light" aria-label="Light"><Sun2Icon size={16} /></ToggleGroupItem>
            <ToggleGroupItem value="dark" aria-label="Dark"><MoonIcon size={16} /></ToggleGroupItem>
          </ToggleGroup>
        </SettingRow>
      </SettingsGroup>

      <SettingsGroup title="Startup">
        <SettingRow title="Start at login" description="Open Airlock when you sign in, so dictation is always ready.">
          <Switch aria-label="Start at login" checked={general.autostart} onCheckedChange={(v) => general.setAutostart(v)} />
        </SettingRow>
        <SettingRow title="Start hidden" description="Launch to the tray without opening the window.">
          <Switch aria-label="Start hidden" checked={general.startHidden} onCheckedChange={(v) => general.setStartHidden(v)} />
        </SettingRow>
        <SettingRow title="Tray icon" description="Show Airlock in the system tray.">
          <Switch aria-label="Tray icon" checked={general.showTray} onCheckedChange={(v) => general.setShowTray(v)} />
        </SettingRow>
      </SettingsGroup>

      <SettingsGroup title="What can leave this computer">
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

      <SettingsGroup title="Storage">
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
          <SettingRow title="Folders" description="Models and engines folders appear once the app is running.">
            <span></span>
          </SettingRow>
        {/each}
        <SettingRow title="Open data folder" description="Chats, attachments and settings live here.">
          <Button size="sm" variant="secondary" onclick={() => commands.openAppDataDir()}>Open</Button>
        </SettingRow>
      </SettingsGroup>
      <p class="-mt-5 px-1 text-sm text-muted-foreground text-pretty">
        Changing a folder does not move files already downloaded.
      </p>

      <SettingsGroup title="About">
        <SettingRow title="Airlock" description="Local AI that stays on your computer. By Nexonauts.">
          <Badge variant="secondary">{version ? `v${version}` : "Preview"}</Badge>
        </SettingRow>
        <SettingRow title="Website" description={APP_URL.replace("https://", "")}>
          <Button size="sm" variant="secondary" onclick={() => openUrl(APP_URL)}>Open</Button>
        </SettingRow>
      </SettingsGroup>
    {/if}
  </main>
</PageScroll>
