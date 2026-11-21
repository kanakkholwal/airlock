<script lang="ts">
  import { PlayIcon, StopIcon } from "@solar-icons/svelte/linear";
  import { tts } from "#lib/tts.svelte.js";
  import {
    VoicePicker,
    VoicePickerSample,
    VoicePickerSelect,
    VoicePickerSpeed,
  } from "#lib/components/agents/voice-picker/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";

  // Decision (product): one card holds everything for a reading: the text, then a single bar with
  // the voice, a sample, the speed, and the main button. Nothing repeats what the page or the
  // player already says. While the voice is being prepared the button shows it in place, so
  // nothing floats over the page.
  let {
    text = $bindable(""),
    speaking,
    preparing,
    ontoggle,
  }: { text: string; speaking: boolean; preparing: boolean; ontoggle: () => void } = $props();

  const voices = $derived(
    (tts.status?.voices ?? []).map((v) => ({
      id: String(v.id),
      name: v.name,
      detail: `${v.accent}, ${v.gender.toLowerCase()}`,
    })),
  );
  const sample = $derived(tts.previewing ? (tts.making > 0 ? "making" : "playing") : "idle");
  const words = $derived(text.trim() ? text.trim().split(/\s+/).length : 0);

  let area = $state<HTMLTextAreaElement>();
  // The box grows with the text up to a limit, then scrolls.
  $effect(() => {
    text;
    if (!area) return;
    area.style.height = "auto";
    area.style.height = `${Math.min(Math.max(area.scrollHeight, 112), 320)}px`;
  });
</script>

<section class="flex flex-col rounded-2xl bg-card p-2 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-300">
  <textarea
    bind:this={area}
    bind:value={text}
    rows={3}
    placeholder="Paste or type what you want to hear"
    aria-label="Text to read aloud"
    class="block w-full resize-none rounded-xl bg-transparent px-3 py-3 text-[15px] leading-relaxed outline-none placeholder:text-muted-foreground [scrollbar-width:thin]"
  ></textarea>

  <div class="flex flex-wrap items-center gap-2 px-1 pt-1 pb-1">
    <VoicePicker
      variant="inline"
      {voices}
      value={String(tts.voice)}
      onValueChange={(id) => tts.setVoice(Number(id))}
      speed={tts.speed}
      onSpeedChange={(speed) => tts.setSpeed(speed)}
      {sample}
      class="min-w-0 flex-1"
    >
      <VoicePickerSelect class="max-w-64 flex-none" />
      <VoicePickerSample onplay={() => tts.preview()} onstop={() => tts.stopPreview()} />
      <VoicePickerSpeed />
    </VoicePicker>

    <div class="ms-auto flex items-center gap-3">
      {#if words > 0}<span class="text-xs text-muted-foreground tabular-nums">{words} {words === 1 ? "word" : "words"}</span>{/if}
      <Button class="min-w-28 gap-2" disabled={!speaking && !text.trim()} onclick={ontoggle}>
        {#if preparing}
          <Spinner size="sm" label="Preparing" />Preparing
        {:else if speaking}
          <StopIcon size={16} />Stop
        {:else}
          <PlayIcon size={16} />Speak
        {/if}
      </Button>
    </div>
  </div>
</section>
