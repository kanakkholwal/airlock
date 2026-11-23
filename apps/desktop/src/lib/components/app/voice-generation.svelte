<script lang="ts">
  import { onMount } from "svelte";
  import { CpuIcon, DownloadMinimalisticIcon, GlobalIcon, SoundwaveIcon } from "@solar-icons/svelte/linear";
  import ConfirmDialog from "#lib/components/app/confirm-dialog.svelte";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";
  import ModelCard from "#lib/components/app/model-card.svelte";
  import ModelMenu from "#lib/components/app/model-menu.svelte";
  import VoiceStudio from "#lib/components/app/voice-studio.svelte";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { formatBytes } from "#lib/format.js";
  import { languageName, languageSummary } from "#lib/languages.js";
  import { llm } from "#lib/llm.svelte.js";
  import { voicegen } from "#lib/voicegen.svelte.js";

  const tiers: Record<string, string> = { tiny: "Tiny", standard: "Standard", studio: "Studio" };
  const featureLabels: Record<string, string> = {
    multilingual: "Multilingual",
    "voice-cloning": "Copies voices",
    "voice-design": "Designs voices",
    "preset-voices": "Preset voices",
  };

  let confirming = $state<string | null>(null);
  onMount(() => void voicegen.init());

  const selected = $derived(llm.voiceModels.find((m) => m.id === voicegen.modelId && m.file.state === "downloaded"));
</script>

<section class="flex flex-col gap-3">
  <h2 class="px-1 text-sm font-medium text-muted-foreground">Voice generation</h2>
  {#if llm.voiceModels.length === 0}
    <p class="px-1 text-sm text-muted-foreground">No voice models found. Restart the app to load the new catalog.</p>
  {/if}
  <ul class="flex flex-col gap-3">
    {#each llm.voiceModels as m (m.id)}
      {@const dl = llm.downloads[m.id]}
      <ModelCard icon={SoundwaveIcon} title={m.name} description={m.description} facts={[m.license]} active={voicegen.loaded === m.id}>
        {#snippet badges()}
          {#if voicegen.loaded === m.id}<Badge size="sm" variant="success">Loaded</Badge>{/if}
          {#if m.recommended}<Badge size="sm">Recommended</Badge>{/if}
          {#if m.tier}<Badge size="sm" variant="secondary">{tiers[m.tier] ?? m.tier}</Badge>{/if}
          {#each m.features as f (f)}
            {#if featureLabels[f]}<Badge size="sm" variant="info">{featureLabels[f]}</Badge>{/if}
          {/each}
        {/snippet}
        {#snippet specs()}
          <span class="inline-flex items-center gap-1.5" title="Disk space">
            <DownloadMinimalisticIcon size={14} />{formatBytes(m.size_bytes)} disk
          </span>
          {#if m.needs_gb}
            <span class="inline-flex items-center gap-1.5" title="Estimated RAM plus GPU memory">
              <CpuIcon size={14} />~{m.needs_gb} GB memory
            </span>
          {/if}
          {#if m.languages.length > 0}
            <span class="inline-flex items-center gap-1.5" title={m.languages.map(languageName).join(", ")}>
              <GlobalIcon size={14} />{languageSummary(m.languages)}
            </span>
          {/if}
        {/snippet}
        {#snippet actions()}
          {#if dl}
            <Button size="sm" variant="secondary" onclick={() => llm.cancelDownload(m.id)}>Cancel</Button>
          {:else if m.file.state === "downloaded"}
            <Button size="sm" variant={voicegen.modelId === m.id ? "secondary" : "default"} onclick={() => voicegen.select(m)}>
              {voicegen.modelId === m.id ? "Open" : "Try it"}
            </Button>
            <ModelMenu items={[{ label: "Delete", onselect: () => (confirming = m.id) }]} />
          {:else}
            <Button size="sm" variant={m.recommended ? "default" : "secondary"} onclick={() => llm.download(m.id)}>
              {m.file.state === "partial" ? "Resume" : "Download"}
            </Button>
          {/if}
        {/snippet}
        {#snippet footer()}
          {#if dl}
            <DownloadProgress compact title="Download" downloaded={dl.downloaded} total={dl.total} bytesPerSecond={dl.bytesPerSecond} phase={dl.phase} />
          {/if}
        {/snippet}
      </ModelCard>
    {/each}
  </ul>

  {#if selected}
    {#key selected.id}
      <VoiceStudio model={selected} />
    {/key}
  {/if}
</section>

<ConfirmDialog
  bind:open={() => confirming !== null, (o) => { if (!o) confirming = null; }}
  title="Delete this voice model?"
  description="Frees the disk space. You can download it again."
  confirmLabel="Delete"
  onconfirm={() => {
    const id = confirming;
    confirming = null;
    if (id) void llm.remove(id);
  }}
/>
