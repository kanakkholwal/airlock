<script lang="ts">
  import { PlayIcon, StopIcon } from "@solar-icons/svelte/linear";
  import { tts } from "#lib/tts.svelte.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Slider } from "#lib/components/ui/slider/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";

  // Decision (product): choosing a voice and hearing it are one gesture. The sample button says
  // what it is doing (making the sample, then playing it), so a short wait never looks like a
  // broken button. Speed and voice changes apply from the next sentence of whatever is playing.
  const items = $derived(
    (tts.status?.voices ?? []).map((v) => ({ value: String(v.id), label: `${v.name} · ${v.accent}, ${v.gender.toLowerCase()}` })),
  );
  const current = $derived(items.find((i) => i.value === String(tts.voice))?.label ?? "Choose a voice");
  const speedLabel = (v: number) => `${v.toFixed(2).replace(/0$/, "")}x`;
  const makingSample = $derived(tts.previewing && tts.making > 0);
</script>

<div class="flex flex-col gap-3">
  <div class="flex items-center justify-between gap-3 rounded-xl bg-muted/60 px-3 py-2">
    <div class="min-w-0">
      <p class="text-sm text-muted-foreground">Voice model</p>
      <p class="truncate text-sm font-medium">{tts.modelName}</p>
    </div>
    <Badge size="sm" variant="secondary">Runs on this computer</Badge>
  </div>
  <div class="flex items-center gap-2">
    <Select items={items} bind:value={() => String(tts.voice), (v) => tts.setVoice(Number(v))}>
      <SelectTrigger class="min-w-0 flex-1"><span class="truncate">{current}</span></SelectTrigger>
      <SelectContent>
        {#each items as i (i.value)}<SelectItem value={i.value}>{i.label}</SelectItem>{/each}
      </SelectContent>
    </Select>
    {#if tts.previewing}
      <Button variant="secondary" class="gap-2" onclick={() => tts.stopPreview()}>
        {#if makingSample}<Spinner size="sm" />Making sample{:else}<StopIcon size={16} />Stop{/if}
      </Button>
    {:else}
      <Button variant="secondary" class="gap-2" onclick={() => tts.preview()}>
        <PlayIcon size={16} />Sample
      </Button>
    {/if}
  </div>
  <div class="flex items-center gap-3">
    <span class="w-10 text-sm text-muted-foreground tabular-nums">{speedLabel(tts.speed)}</span>
    <Slider min={0.5} max={2} step={0.05} label="Speaking speed" bind:value={() => tts.speed, (v) => tts.setSpeed(v)} />
  </div>
</div>
