<script lang="ts">
  import { PauseIcon, PlayIcon, StopIcon, Tuning2Icon } from "@solar-icons/svelte/linear";
  import { speaker } from "#lib/speak.svelte.js";
  import { tts } from "#lib/tts.svelte.js";
  import VoicePicker from "#lib/components/app/voice-picker.svelte";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Popover, PopoverContent, PopoverTrigger } from "#lib/components/ui/popover/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";
  import { Tooltip, TooltipContent, TooltipTrigger } from "#lib/components/ui/tooltip/index.js";

  // Decision (product): while anything is being read aloud, a slim row at the top of the page holds
  // the controls, so you can pause, stop or change the voice without going back to the message.
  // It sits in the layout, not over it, so it never covers a title or a button.
  // It says honestly what is happening: getting the voice ready, speaking which sentence, paused.
  // `inline` is for a page that already has its own place for it, such as under the Voice orb.
  let { inline = false }: { inline?: boolean } = $props();
  const active = $derived(speaker.speakingId !== null);
  const label = $derived(
    speaker.preparing
      ? "Getting the voice ready"
      : speaker.paused
        ? "Paused"
        : speaker.engine === "natural"
          ? `Reading aloud · ${tts.voiceName}`
          : "Reading aloud",
  );
  const progress = $derived(
    speaker.position.of > 1 && speaker.position.at > 0 ? `${speaker.position.at} of ${speaker.position.of}` : "",
  );
</script>

{#if active}
  <div class="flex shrink-0 justify-center {inline ? '' : 'px-4 pt-2'} motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-2 motion-safe:duration-200">
  <div class="flex max-w-full items-center gap-1 rounded-full bg-muted py-1 ps-3 pe-1" role="status" aria-live="polite">
    <span class="flex h-5 w-5 items-center justify-center" aria-hidden="true">
      {#if speaker.preparing}
        <Spinner size="sm" />
      {:else}
        <span class="flex h-4 items-end gap-0.5 {speaker.paused ? 'opacity-40' : ''}">
          {#each [0, 1, 2, 3] as i (i)}
            <i
              class="w-0.5 rounded-full bg-primary {speaker.paused ? 'h-1.5' : 'motion-safe:animate-pulse'}"
              style:height={speaker.paused ? undefined : `${8 + ((i * 5) % 8)}px`}
              style:animation-delay="{i * 120}ms"
            ></i>
          {/each}
        </span>
      {/if}
    </span>
    <span class="ms-1 me-2 text-sm whitespace-nowrap">{label}</span>
    {#if progress}<span class="me-1 text-sm text-muted-foreground tabular-nums">{progress}</span>{/if}

    {#if !speaker.preparing}
      <Tooltip>
        <TooltipTrigger>
          {#snippet child({ props })}
            <Button
              {...props}
              variant="ghost"
              size="icon-sm"
              aria-label={speaker.paused ? "Resume" : "Pause"}
              onclick={() => (speaker.paused ? speaker.resume() : speaker.pause())}
            >
              {#if speaker.paused}<PlayIcon size={16} />{:else}<PauseIcon size={16} />{/if}
            </Button>
          {/snippet}
        </TooltipTrigger>
        <TooltipContent>{speaker.paused ? "Resume" : "Pause"}</TooltipContent>
      </Tooltip>
    {/if}

    <Tooltip>
      <TooltipTrigger>
        {#snippet child({ props })}
          <Button {...props} variant="ghost" size="icon-sm" aria-label="Stop" onclick={() => speaker.stop()}>
            <StopIcon size={16} />
          </Button>
        {/snippet}
      </TooltipTrigger>
      <TooltipContent>Stop</TooltipContent>
    </Tooltip>

    {#if tts.ready}
      <Popover>
        <PopoverTrigger>
          {#snippet child({ props })}
            <Button {...props} variant="ghost" size="icon-sm" aria-label="Voice and speed">
              <Tuning2Icon size={16} />
            </Button>
          {/snippet}
        </PopoverTrigger>
        <PopoverContent align="center" class="w-80">
          <p class="mb-3 text-sm text-muted-foreground">Changes apply from the next sentence.</p>
          <VoicePicker />
        </PopoverContent>
      </Popover>
    {/if}
  </div>
  </div>
{/if}
