<script lang="ts">
  import { Microphone3Icon } from "@solar-icons/svelte/linear";
  import { speech } from "#lib/speech.svelte.js";
  import ConfirmDialog from "#lib/components/app/confirm-dialog.svelte";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import ModelCard from "#lib/components/app/model-card.svelte";
  import ModelMenu from "#lib/components/app/model-menu.svelte";
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
    <ModelCard
      icon={Microphone3Icon}
      title={m.name}
      description={m.description}
      facts={[size(m.size_mb), languages(m)]}
      active={speech.current === m.id}
    >
      {#snippet badges()}
        {#if speech.current === m.id}<Badge size="sm" variant="success">In use</Badge>{/if}
        {#if m.is_recommended}<Badge size="sm">Recommended</Badge>{/if}
        {#if m.supports_streaming}<Badge size="sm" variant="info">Live transcript</Badge>{/if}
      {/snippet}
      {#snippet actions()}
        {#if p || m.is_downloading}
          <Button size="sm" variant="secondary" onclick={() => speech.cancel(m.id)}>Cancel</Button>
        {:else if m.is_downloaded}
          {#if speech.current !== m.id}
            <Button size="sm" variant="secondary" onclick={() => speech.use(m.id)}>Use</Button>
          {/if}
          <ModelMenu
            items={[
              {
                label: "Delete",
                onselect: () => {
                  deleting = m;
                  deleteOpen = true;
                },
              },
            ]}
          />
        {:else}
          <Button size="sm" variant="secondary" onclick={() => speech.download(m.id)}>
            {m.partial_size > 0 ? "Resume" : "Download"}
          </Button>
        {/if}
      {/snippet}
      {#snippet footer()}
        {#if p}
          <DownloadProgress compact title={m.name} downloaded={p.downloaded} total={p.total} bytesPerSecond={p.bytesPerSecond ?? 0} />
        {/if}
      {/snippet}
    </ModelCard>
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
