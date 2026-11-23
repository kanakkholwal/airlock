<script lang="ts">
  import { onMount } from "svelte";
  import {
    DangerTriangleIcon,
    LockKeyholeIcon,
    VolumeLoudIcon,
  } from "@solar-icons/svelte/linear";
  import { inTauri } from "#lib/env.js";
  import { formatBytes } from "#lib/format.js";
  import { speaker } from "#lib/speak.svelte.js";
  import { tts } from "#lib/tts.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import { voicegen } from "#lib/voicegen.svelte.js";
  import { languageSummary } from "#lib/languages.js";
  import VoiceStudio from "#lib/components/app/voice-studio.svelte";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import VoiceControl from "#lib/components/app/voice-control.svelte";
  import VoiceComposer from "#lib/components/app/voice-composer.svelte";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import FluidOrb from "#lib/components/app/fluid-orb.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Button } from "#lib/components/ui/button/index.js";

  // Decision (product): Voice is where you turn text you have into speech: paste it, pick a
  // voice, listen. Chat replies use the same voice and speed through the speaker button, so
  // there is one set of choices, kept here and on Models.
  const ID = "voice-page";
  let text = $state("");

  onMount(() => {
    void tts.init();
    void voicegen.init();
  });

  // Speech models that are on disk, offered next to the natural read-aloud voices.
  const generators = $derived(llm.voiceModels.filter((m) => m.file.state === "downloaded"));
  const chosen = $derived(generators.find((m) => m.id === voicegen.source));
  const choices = $derived([
    { value: "natural", label: "Natural voices" },
    ...generators.map((m) => ({ value: m.id, label: m.name })),
  ]);
  const chosenLabel = $derived(choices.find((c) => c.value === (chosen ? chosen.id : "natural"))?.label ?? "Natural voices");

  // The studio edits one model at a time; keep it pointed at the model picked here.
  $effect(() => {
    if (chosen) voicegen.select(chosen);
  });

  const status = $derived(tts.status);
  const speaking = $derived(speaker.speakingId === ID);

  function toggle() {
    if (speaking) speaker.stop();
    else if (text.trim()) speaker.speak(text, ID);
  }
</script>

<PageScroll>
  <div class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-12">
    <PageHeader title="Voice" />

    {#if inTauri()}
      <div class="flex flex-wrap items-center justify-between gap-3 rounded-2xl bg-card px-4 py-3">
        <div class="min-w-0">
          <div class="text-sm font-medium">Voice model</div>
          <div class="text-xs text-muted-foreground">
            {#if chosen}{languageSummary(chosen.languages)}{:else if generators.length === 0}Download a voice model for voice design, cloning and more languages.{:else}Read-aloud voices, ready to use{/if}
          </div>
        </div>
        <span class="flex items-center gap-2">
          <Select items={choices} bind:value={() => (chosen ? chosen.id : "natural"), (v) => voicegen.setSource(v as string)}>
            <SelectTrigger class="w-52"><span class="truncate">{chosenLabel}</span></SelectTrigger>
            <SelectContent align="end">
              {#each choices as c (c.value)}<SelectItem value={c.value}>{c.label}</SelectItem>{/each}
            </SelectContent>
          </Select>
          <Button size="sm" variant="secondary" href="/models?tab=voice">More voices</Button>
        </span>
      </div>
    {/if}

    {#if chosen}
      {#key chosen.id}
        <VoiceStudio model={chosen} />
      {/key}
    {:else if !inTauri()}
      <StateView
        icon={VolumeLoudIcon}
        title="Open the Airlock app to use voices"
      />
    {:else if !status}
      <CardSkeleton count={2} />
    {:else if !status.supported}
      <StateView
        icon={DangerTriangleIcon}
        tone="warning"
        title="Natural voices are not available on this system yet"
      />
    {:else if !status.ready}
      <StateView
        icon={VolumeLoudIcon}
        tone="primary"
        variant="card"
        title="Get natural voices"
        description={tts.installing
          ? "Downloading. You can keep using Airlock while this carries on."
          : `One download of about ${formatBytes(status.missing_bytes)} adds ${status.voices.length} clear English voices. They run on this computer.`}
      >
        {#snippet actions()}
          {#if tts.progress}
            <div class="w-full max-w-sm">
              <DownloadProgress
                compact
                title="Natural voices"
                downloaded={tts.progress.downloaded}
                total={tts.progress.total}
                bytesPerSecond={tts.progress.bytesPerSecond}
                phase={tts.progress.phase}
                oncancel={() => tts.cancelInstall()}
              />
            </div>
          {:else}
            <Button class="gap-2" onclick={() => tts.install()}>
              <VolumeLoudIcon size={16} />Download {formatBytes(status.missing_bytes)}
            </Button>
            <span class="flex items-center gap-1.5 text-sm text-muted-foreground">
              <LockKeyholeIcon size={14} />Private, and checked on download
            </span>
          {/if}
        {/snippet}
      </StateView>
    {:else}
      <section class="flex flex-col items-center gap-4 pt-2 pb-4 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
        <FluidOrb
          size={148}
          colors={["#1A73F2", "#7C5CFF", "#14B8A6"]}
          activity={speaking && !speaker.preparing && !speaker.paused ? 0.6 : 0}
          label={speaker.paused ? "Paused" : speaking ? "Speaking" : "Ready to speak"}
        />
        <p class="flex h-6 items-center gap-2 text-sm" aria-live="polite">
          {#if speaker.preparing}
            <span class="text-shimmer font-medium">Preparing voice</span>
          {:else if speaking && speaker.paused}
            <span class="font-medium">Paused</span>
          {:else if speaking}
            <span class="font-medium">Speaking</span>
          {:else}
            <span class="font-medium">Ready</span>
          {/if}
          <span class="text-muted-foreground">{tts.voiceName}</span>
        </p>
        {#if speaker.speakingId !== null && !speaker.preparing}
          <VoiceControl inline />
        {/if}
      </section>

      <VoiceComposer bind:text {speaking} preparing={speaking && speaker.preparing} ontoggle={toggle} />

      {#if speaker.error || tts.error}
        <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription class="whitespace-pre-line">{speaker.error ?? tts.error}</AlertDescription></Alert>
      {/if}
    {/if}
  </div>
</PageScroll>
