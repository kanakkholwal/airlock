<script lang="ts">
  import { Microphone3Icon } from "@solar-icons/svelte/linear";
  import { speech } from "#lib/speech.svelte.js";
  import ConfirmDialog from "#lib/components/app/confirm-dialog.svelte";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";
  import type { ModelInfo } from "../../../bindings";

  // Decision (product): Handy ships 70+ speech models. Five are recommended, so those show by
  // default and the rest sit behind "More models". The model in use is marked, not hidden.
  let showAll = $state(false);
  let deleting = $state<ModelInfo | null>(null);
  let deleteOpen = $state(false);

  const list = $derived(showAll ? [...speech.recommended, ...speech.others] : speech.recommended);

  const size = (mb: number) => (mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`);
  const mb = (b: number) => `${Math.round(b / 1_048_576)} MB`;
  const languages = (m: ModelInfo) =>
    m.supported_languages.length > 1 ? `${m.supported_languages.length} languages` : "English";
</script>

<ul class="flex flex-col gap-3">
  {#each list as m (m.id)}
    {@const p = speech.progress[m.id]}
    <li class="flex flex-col gap-3 rounded-2xl bg-card p-4 transition-colors duration-150 hover:bg-muted/60">
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <Microphone3Icon size={16} class="shrink-0 text-muted-foreground" />
            <span class="font-[450]">{m.name}</span>
            {#if m.is_recommended}<Badge size="sm">Recommended</Badge>{/if}
            {#if m.supports_streaming}<Badge size="sm" variant="info">Live transcript</Badge>{/if}
            {#if speech.current === m.id}<Badge size="sm" variant="success">In use</Badge>{/if}
          </div>
          <p class="mt-1 text-sm text-muted-foreground text-pretty">{m.description}</p>
          <p class="mt-1 text-sm text-muted-foreground">{size(m.size_mb)} · {languages(m)}</p>
        </div>
        <div class="flex shrink-0 gap-2">
          {#if p || m.is_downloading}
            <Button size="sm" variant="secondary" onclick={() => speech.cancel(m.id)}>Cancel</Button>
          {:else if m.is_downloaded}
            {#if speech.current !== m.id}
              <Button size="sm" variant="secondary" onclick={() => speech.use(m.id)}>Use</Button>
            {/if}
            <Button
              size="sm"
              variant="ghost"
              onclick={() => {
                deleting = m;
                deleteOpen = true;
              }}
            >
              Delete
            </Button>
          {:else}
            <Button size="sm" variant="secondary" onclick={() => speech.download(m.id)}>
              {m.partial_size > 0 ? "Resume" : "Download"}
            </Button>
          {/if}
        </div>
      </div>
      {#if p}
        <DownloadProgress compact title={m.name} downloaded={p.downloaded} total={p.total} bytesPerSecond={p.bytesPerSecond ?? 0} />
      {/if}
    </li>
  {/each}
</ul>

{#if speech.others.length > 0}
  <div>
    <Button variant="ghost" size="sm" onclick={() => (showAll = !showAll)}>
      {showAll ? "Show fewer models" : `More models (${speech.others.length})`}
    </Button>
  </div>
{/if}

<ConfirmDialog
  bind:open={deleteOpen}
  title={`Delete ${deleting?.name ?? "this model"}?`}
  description={`It frees ${deleting ? size(deleting.size_mb) : ""} of disk. You can download it again later.`}
  confirmLabel="Delete"
  onconfirm={async () => {
    if (deleting) await speech.remove(deleting.id);
  }}
/>
