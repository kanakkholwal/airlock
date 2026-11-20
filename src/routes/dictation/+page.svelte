<script lang="ts">
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import { onMount } from "svelte";
  import type { OverlayStyle, ShortcutActivation } from "../../bindings";
  import { inTauri } from "#lib/env.js";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import SettingsSkeleton from "#lib/components/app/settings-skeleton.svelte";
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import SettingsGroup from "#lib/components/app/settings-group.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { ShortcutRecorder } from "#lib/components/ui/shortcut-recorder/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";
  import { dictation } from "#lib/settings.svelte.js";

  const activations: { value: ShortcutActivation; label: string }[] = [
    { value: "hold_or_toggle", label: "Hold or tap" },
    { value: "push_to_talk", label: "Hold to talk" },
    { value: "toggle", label: "Press to start and stop" },
  ];
  const overlays: { value: OverlayStyle; label: string }[] = [
    { value: "minimal", label: "Minimal pill" },
    { value: "live", label: "Live transcript" },
    { value: "none", label: "Hidden" },
  ];

  onMount(() => dictation.load());
</script>

<PageScroll>
<main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-16">
  <PageHeader title="Dictation" description="Hold a key, speak, and the text lands where you were typing." />

  {#if !inTauri()}
    <Alert variant="info">
      <AlertDescription>This is a browser preview, so dictation settings are not saved. Open the desktop app to change them.</AlertDescription>
    </Alert>
  {/if}

  {#if dictation.error}
    <Alert variant="destructive"><AlertDescription>{dictation.error}</AlertDescription></Alert>
  {/if}

  {#if !dictation.loaded && inTauri() && !dictation.error}
    <SettingsSkeleton groups={2} rows={2} />
  {:else}

  <SettingsGroup title="Shortcut">
    <SettingRow title="Dictation shortcut" description="Works in any app.">
      <ShortcutRecorder
        label="Dictation shortcut"
        value={dictation.shortcut}
        defaultValue="ctrl+space"
        onValueChange={(v) => dictation.setShortcut(v)}
      />
    </SettingRow>
    <SettingRow title="Activation" description="How the shortcut starts and stops recording.">
      <Select
        items={activations}
        bind:value={() => dictation.activation, (v) => dictation.setActivation(v as ShortcutActivation)}
      >
        <SelectTrigger class="w-56">
          <span class="truncate">{activations.find((a) => a.value === dictation.activation)?.label}</span>
        </SelectTrigger>
        <SelectContent align="end">
          {#each activations as a (a.value)}
            <SelectItem value={a.value}>{a.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
    </SettingRow>
  </SettingsGroup>

  <SettingsGroup title="Output">
    <SettingRow title="Recording overlay" description="What shows while you speak.">
      <Select
        items={overlays}
        bind:value={() => dictation.overlayStyle, (v) => dictation.setOverlayStyle(v as OverlayStyle)}
      >
        <SelectTrigger class="w-56">
          <span class="truncate">{overlays.find((o) => o.value === dictation.overlayStyle)?.label}</span>
        </SelectTrigger>
        <SelectContent align="end">
          {#each overlays as o (o.value)}
            <SelectItem value={o.value}>{o.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
    </SettingRow>
    <SettingRow title="Clean up with a model" description="Fixes punctuation and filler words after you speak.">
      <Switch
        aria-label="Clean up with a model"
        checked={dictation.cleanupWithLlm}
        onCheckedChange={(v) => dictation.setCleanup(v)}
      />
    </SettingRow>
  </SettingsGroup>
{/if}
</main>
</PageScroll>
