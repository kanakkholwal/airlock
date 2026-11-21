<script lang="ts">
  import { onMount } from "svelte";
  import {
    DangerTriangleIcon,
    LockKeyholeIcon,
    PlayIcon,
    StopIcon,
    VolumeLoudIcon,
  } from "@solar-icons/svelte/linear";
  import { inTauri } from "#lib/env.js";
  import { formatBytes } from "#lib/format.js";
  import { speaker } from "#lib/speak.svelte.js";
  import { tts } from "#lib/tts.svelte.js";
  import VoiceControl from "#lib/components/app/voice-control.svelte";
  import VoicePicker from "#lib/components/app/voice-picker.svelte";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { VoiceOrb } from "#lib/components/agents/voice-orb/index.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Textarea } from "#lib/components/ui/textarea/index.js";

  // Decision (product): Voice is where you turn text you have into speech: paste it, pick a
  // voice, listen. Chat replies use the same voice and speed through the speaker button, so
  // there is one set of choices, kept here and on Models.
  const ID = "voice-page";
  const samples = [
    "Welcome back. Everything you hear is made on this computer, and nothing leaves it.",
    "The quick brown fox jumps over the lazy dog, and then takes a well-earned nap.",
  ];

  let text = $state("");

  onMount(() => void tts.init());

  const status = $derived(tts.status);
  const speaking = $derived(speaker.speakingId === ID);
  const words = $derived(text.trim() ? text.trim().split(/\s+/).length : 0);

  function toggle() {
    if (speaking) speaker.stop();
    else if (text.trim()) speaker.speak(text, ID);
  }
</script>

<PageScroll>
  <div class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-12">
    <PageHeader title="Voice" description="Turn any text into natural speech, made on this computer." />

    {#if !inTauri()}
      <StateView
        icon={VolumeLoudIcon}
        title="Open the Airlock app to use voices"
        description="Voices run on your computer, so they need the desktop app."
      />
    {:else if !status}
      <CardSkeleton count={2} />
    {:else if !status.supported}
      <StateView
        icon={DangerTriangleIcon}
        tone="warning"
        title="Natural voices are not available on this system yet"
        description="Reading chat replies aloud still works with the voices that come with your system."
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
      <section class="flex flex-col items-center gap-2 py-2 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
        <VoiceOrb
          size="default"
          variant="ember"
          active={speaking && !speaker.paused}
          activity={speaking && !speaker.preparing && !speaker.paused ? 0.4 : 0}
          label={speaker.paused ? "Paused" : speaking ? "Speaking" : "Ready to speak"}
        />
        {#if speaker.speakingId !== null}
          <VoiceControl inline />
        {:else}
          <p class="text-sm text-muted-foreground" aria-live="polite">Ready · {tts.modelName} · {tts.voiceName}</p>
        {/if}
      </section>

      <section class="flex flex-col gap-3 rounded-2xl bg-card p-3">
        <Textarea
          bind:value={text}
          rows={7}
          placeholder="Paste or type what you want to hear"
          aria-label="Text to read aloud"
        />
        <div class="flex flex-wrap items-center gap-3">
          <div class="min-w-72 flex-1"><VoicePicker /></div>
          <div class="ms-auto flex items-center gap-3">
            {#if words > 0}<Badge size="sm" variant="secondary">{words} words</Badge>{/if}
            <Button class="gap-2" disabled={!speaking && !text.trim()} onclick={toggle}>
              {#if speaking}<StopIcon size={16} />Stop{:else}<PlayIcon size={16} />Speak{/if}
            </Button>
          </div>
        </div>
      </section>

      {#if !text.trim()}
        <div class="flex flex-wrap justify-center gap-2">
          {#each samples as sample (sample)}
            <Button size="sm" variant="secondary" class="transition-transform hover:-translate-y-0.5" onclick={() => (text = sample)}>
              {sample.length > 48 ? `${sample.slice(0, 48)}…` : sample}
            </Button>
          {/each}
        </div>
      {/if}

      {#if speaker.error || tts.error}
        <Alert variant="destructive"><AlertDescription class="whitespace-pre-line">{speaker.error ?? tts.error}</AlertDescription></Alert>
      {/if}
    {/if}
  </div>
</PageScroll>
