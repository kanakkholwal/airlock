<script lang="ts">
  import { Tuning2Icon } from "@solar-icons/svelte/linear";
  import { speaker } from "#lib/speak.svelte.js";
  import { tts } from "#lib/tts.svelte.js";
  import VoicePicker from "#lib/components/app/voice-picker.svelte";
  import {
    SpeechPlayer,
    SpeechPlayerControls,
    SpeechPlayerPlay,
    SpeechPlayerPosition,
    SpeechPlayerProgress,
    SpeechPlayerRate,
    SpeechPlayerStop,
  } from "#lib/components/agents/speech-player/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Popover, PopoverContent, PopoverTrigger } from "#lib/components/ui/popover/index.js";

  // Decision (product): while anything is being read aloud, the Baby UI Speech Player sits in a
  // slim row at the top of the page, so you can pause, stop or change speed and voice without
  // going back to the message. It sits in the layout, not over it, so it never covers a title.
  // `inline` is for a page that already has its own place for it, such as under the Voice orb.
  let { inline = false }: { inline?: boolean } = $props();

  const active = $derived(speaker.speakingId !== null);
  const state = $derived(
    speaker.preparing ? "preparing" : speaker.paused ? "paused" : active ? "playing" : "idle",
  );
</script>

{#if active}
  <div class="flex shrink-0 justify-center {inline ? '' : 'px-4 pt-2'} motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-2 motion-safe:duration-200">
    <div class="flex w-full max-w-md items-start gap-1">
      <SpeechPlayer
        variant="inline"
        {state}
        at={speaker.position.at}
        count={speaker.position.of}
        rate={tts.speed}
        onRateChange={(rate) => tts.setSpeed(rate)}
        class="flex-1"
      >
        <SpeechPlayerControls>
          <SpeechPlayerPlay onclick={() => (speaker.paused ? speaker.resume() : speaker.pause())} />
          <SpeechPlayerStop onclick={() => speaker.stop()} />
          <SpeechPlayerPosition />
          <SpeechPlayerRate />
        </SpeechPlayerControls>
        <SpeechPlayerProgress />
      </SpeechPlayer>
      <!-- The Voice page has its own voice and speed bar, so the tuning button is for other pages. -->
      {#if tts.ready && !inline}
        <Popover>
          <PopoverTrigger>
            {#snippet child({ props })}
              <Button {...props} variant="ghost" size="icon" aria-label="Voice and speed">
                <Tuning2Icon size={16} />
              </Button>
            {/snippet}
          </PopoverTrigger>
          <PopoverContent align="end" class="w-80">
            <p class="mb-3 text-sm text-muted-foreground">Changes apply from the next sentence.</p>
            <VoicePicker />
          </PopoverContent>
        </Popover>
      {/if}
    </div>
  </div>
{/if}
