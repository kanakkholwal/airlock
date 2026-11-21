<script lang="ts">
  import { MicrophoneIcon, StarIcon, TrashBinMinimalisticIcon } from "@solar-icons/svelte/linear";
  import { StarIcon as StarFilledIcon } from "@solar-icons/svelte/bold";
  import { speech } from "#lib/speech.svelte.js";
  import ConfirmDialog from "#lib/components/app/confirm-dialog.svelte";
  import TranscriptView from "#lib/components/app/transcript-view.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Button } from "#lib/components/ui/button/index.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";

  // Decision (product): history exists so a dictation is never lost when the paste goes to the
  // wrong place. Copy is the main action; starring keeps a line you want to find again.
  let deletingId = $state<number | null>(null);
  let deleteOpen = $state(false);

  // Timestamps are seconds since the epoch.
  const when = (seconds: number) =>
    new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(new Date(seconds * 1000));
  const textOf = (e: (typeof speech.history)[number]) => e.post_processed_text || e.transcription_text;
</script>

{#if !speech.historyLoaded}
  <div class="flex flex-col gap-3" role="status" aria-label="Loading history">
    {#each [0, 1, 2] as i (i)}
      <div class="flex flex-col gap-2.5 rounded-2xl bg-card p-4">
        <Skeleton class="h-3.5 w-32" />
        <Skeleton class="h-4 w-full" />
        <Skeleton class="h-4 w-2/3" />
      </div>
    {/each}
  </div>
{:else if speech.history.length === 0}
  <StateView
    icon={MicrophoneIcon}
    title="No dictations yet"
    description="Hold your shortcut in any app and speak. What you say shows up here, so nothing is lost if a paste goes astray."
    variant="card"
  />
{:else}
  <ul class="flex flex-col gap-3">
    {#each speech.history as e (e.id)}
      <li class="flex flex-col gap-2 rounded-2xl bg-card p-4 transition-colors duration-150 hover:bg-muted/60 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1 motion-safe:duration-300">
        <div class="flex items-center justify-between gap-3">
          <span class="text-sm text-muted-foreground tabular-nums">{when(e.timestamp)}</span>
          <div class="flex items-center gap-0.5">
            <Button
              variant="ghost"
              size="icon-sm"
              aria-pressed={e.saved}
              aria-label={e.saved ? "Remove star" : "Star this dictation"}
              onclick={() => speech.toggleSaved(e.id)}
            >
              {#if e.saved}<StarFilledIcon size={16} class="text-warning-strong" />{:else}<StarIcon size={16} />{/if}
            </Button>
            <CopyButton text={textOf(e)} iconOnly label="Copy text" />
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Delete this dictation"
              onclick={() => {
                deletingId = e.id;
                deleteOpen = true;
              }}
            >
              <TrashBinMinimalisticIcon size={16} />
            </Button>
          </div>
        </div>
        <TranscriptView entry={e} />
      </li>
    {/each}
  </ul>
  {#if speech.hasMoreHistory}
    <div><Button variant="secondary" onclick={() => speech.loadHistory()}>Load older</Button></div>
  {/if}
{/if}

<ConfirmDialog
  bind:open={deleteOpen}
  title="Delete this dictation?"
  description="The text and its recording are removed from this computer. This cannot be undone."
  onconfirm={async () => {
    if (deletingId !== null) await speech.removeEntry(deletingId);
  }}
/>
