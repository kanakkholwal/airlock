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
  <p class="px-1 text-sm text-muted-foreground text-pretty">
    Read-aloud voices speak chat replies out loud. They run on this computer, so nothing you read is sent anywhere.
  </p>

  {#if !status}
    <div class="h-32 animate-pulse rounded-2xl bg-card" role="status" aria-label="Loading"></div>
  {:else if !status.supported}
    <StateView
      icon={DangerTriangleIcon}
      tone="warning"
      title="Natural voices are not available on this system yet"
      description="Reading aloud still works with the voices that come with your system."
    />
  {:else}
    <div class="flex flex-col gap-4 rounded-2xl bg-card p-4">
      <div class="flex items-start gap-3">
        <span class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <VolumeLoudIcon size={18} />
        </span>
        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-2">
            <span class="font-medium">Kokoro natural voices</span>
            <Badge size="sm">Recommended</Badge>
            {#if status.ready}<Badge size="sm" variant="success">Installed</Badge>{/if}
          </div>
          <p class="mt-1 text-sm text-muted-foreground text-pretty">
            28 clear English voices, American and British. {formatBytes(status.total_bytes)} · Apache-2.0
          </p>
        </div>
        {#if status.ready && !tts.installing}
          <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => (confirmRemove = true)}>
            <TrashBin2Icon size={16} />Remove
          </Button>
        {/if}
      </div>

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
      {:else if !status.ready}
        <div class="flex flex-wrap items-center gap-3">
          <Button class="gap-2" onclick={() => tts.install()}>
            <VolumeLoudIcon size={16} />Download {formatBytes(status.missing_bytes)}
          </Button>
          <span class="flex items-center gap-1.5 text-sm text-muted-foreground">
            <LockKeyholeIcon size={14} />Checked on download
          </span>
        </div>
      {:else}
        <div class="max-w-md motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300"><VoicePicker /></div>
      {/if}
    </div>
  {/if}

  {#if tts.error}
    <Alert variant="destructive"><AlertDescription class="whitespace-pre-line">{tts.error}</AlertDescription></Alert>
  {/if}
</section>

<ConfirmDialog
  bind:open={confirmRemove}
  title="Remove natural voices?"
  description={`This frees ${formatBytes(status?.total_bytes ?? 0)}. Reading aloud goes back to your system voice, and you can download the voices again.`}
  confirmLabel="Remove"
  onconfirm={() => tts.remove()}
/>
