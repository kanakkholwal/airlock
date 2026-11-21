<script lang="ts">
  import { onMount } from "svelte";
  import {
    DangerTriangleIcon,
    LockKeyholeIcon,
    TrashBin2Icon,
    VolumeLoudIcon,
  } from "@solar-icons/svelte/linear";
  import { formatBytes } from "#lib/format.js";
  import { tts } from "#lib/tts.svelte.js";
  import VoicePicker from "#lib/components/app/voice-picker.svelte";
  import ConfirmDialog from "#lib/components/app/confirm-dialog.svelte";
  import ModelCard from "#lib/components/app/model-card.svelte";
  import ModelMenu from "#lib/components/app/model-menu.svelte";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";

  // Decision (product): reading aloud works out of the box with the system voice. This card is
  // the upgrade: one download of natural voices that also run on this computer. Hearing a voice
  // comes before choosing it, so Preview sits right next to the picker.
  let confirmRemove = $state(false);

  onMount(() => void tts.init());

  const status = $derived(tts.status);
</script>

<section class="flex flex-col gap-3">
  {#if !status}
    <div class="h-32 animate-pulse rounded-2xl bg-card" role="status" aria-label="Loading"></div>
  {:else if !status.supported}
    <StateView
      icon={DangerTriangleIcon}
      tone="warning"
      title="Natural voices are not available on this system yet"
    />
  {:else}
    <ul class="flex flex-col gap-3">
      <ModelCard
        icon={VolumeLoudIcon}
        title="Kokoro natural voices"
        description="28 clear English voices, American and British."
        facts={[formatBytes(status.total_bytes), "Apache-2.0"]}
        active={status.ready}
      >
        {#snippet badges()}
          {#if status.ready}<Badge size="sm" variant="success">Installed</Badge>{/if}
          <Badge size="sm">Recommended</Badge>
        {/snippet}
        {#snippet actions()}
          {#if status.ready && !tts.installing}
            <ModelMenu items={[{ label: "Remove", onselect: () => (confirmRemove = true) }]} />
          {:else if !status.ready && !tts.progress}
            <Button size="sm" onclick={() => tts.install()}>Download {formatBytes(status.missing_bytes)}</Button>
          {/if}
        {/snippet}
        {#snippet footer()}
          {#if tts.progress}
            <DownloadProgress
              compact
              title="Natural voices"
              downloaded={tts.progress.downloaded}
              total={tts.progress.total}
              bytesPerSecond={tts.progress.bytesPerSecond}
              phase={tts.progress.phase}
              oncancel={() => tts.cancelInstall()}
            />
          {:else if status.ready}
            <div class="max-w-md motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300"><VoicePicker /></div>
          {:else}
            <span class="flex items-center gap-1.5 text-xs text-muted-foreground"><LockKeyholeIcon size={14} />Checked on download</span>
          {/if}
        {/snippet}
      </ModelCard>
    </ul>
    {/if}

  {#if tts.error}
    <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription class="whitespace-pre-line">{tts.error}</AlertDescription></Alert>
  {/if}
</section>

<ConfirmDialog
  bind:open={confirmRemove}
  title="Remove natural voices?"
  description={`Frees ${formatBytes(status?.total_bytes ?? 0)}. You can download it again.`}
  confirmLabel="Remove"
  onconfirm={() => tts.remove()}
/>
