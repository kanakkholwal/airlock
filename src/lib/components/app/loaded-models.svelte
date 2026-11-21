<script lang="ts">
  import type { Component } from "svelte";
  import {
    ChatLineIcon,
    GalleryIcon,
    Microphone3Icon,
    PauseIcon,
    PlayIcon,
    PowerIcon,
    SoundwaveIcon,
    SpeedometerLowIcon,
    StopCircleIcon,
    Tuning2Icon,
  } from "@solar-icons/svelte/linear";
  import { llm } from "#lib/llm.svelte.js";
  import { images } from "#lib/images.svelte.js";
  import { speaker } from "#lib/speak.svelte.js";
  import { speech } from "#lib/speech.svelte.js";
  import { tts } from "#lib/tts.svelte.js";
  import VoicePicker from "#lib/components/app/voice-picker.svelte";
  import { rateRealtime, rateSpeed, speedLabel, type SpeedRating } from "#lib/speed.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Popover, PopoverContent, PopoverTrigger } from "#lib/components/ui/popover/index.js";
  import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#lib/components/ui/collapsible/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";

  // Decision (product): one place, on every screen, that answers "what is using my memory right
  // now, and can I free it". Each row is a model that is loaded, with the two things you can do
  // to it: measure it and unload it. Nothing loaded is a normal state, so it says so calmly.
  const tone: Record<SpeedRating, "success" | "info" | "warning" | "destructive"> = {
    excellent: "success",
    decent: "info",
    usable: "warning",
    slow: "destructive",
  };
  const fill: Record<SpeedRating, string> = {
    excellent: "bg-success",
    decent: "bg-info",
    usable: "bg-warning",
    slow: "bg-destructive",
  };

  const chatState = $derived(llm.engine.state);
  const chat = $derived(
    (chatState === "ready" || chatState === "loading") && llm.current
      ? { name: llm.current.name, loading: chatState === "loading" }
      : null,
  );
  const chatSpeed = $derived(llm.speed && llm.speed.model === llm.current?.id ? llm.speed.value : null);

  const speechName = $derived(speech.models.find((m) => m.id === speech.loadedModel)?.name ?? null);
  const dictation = $derived(
    speechName ? { name: speechName, loading: false } : speech.loadingModel ? { name: "Speech model", loading: true } : null,
  );
  const dictationSpeed = $derived(
    speech.lastRun && speech.lastRun.waitSeconds > 0 ? speech.lastRun.audioSeconds / speech.lastRun.waitSeconds : null,
  );

  const making = $derived(images.run !== null);

  // Models that are set up but not in memory right now, so they can be loaded from here.
  const chatIdle = $derived(
    !chat && chatState !== "failed" && llm.engines.some((e) => e.installed) && llm.downloaded.length > 0,
  );
  const speechIdleName = $derived(
    !dictation && speech.current ? (speech.models.find((m) => m.id === speech.current)?.name ?? null) : null,
  );

  // The voice engine is not resident: it starts for each sentence and exits. So its state is
  // what it is doing, not whether it is loaded.
  const voiceReady = $derived(tts.ready);
  const voiceBusy = $derived(speaker.speakingId !== null || tts.making > 0);
  const voiceState = $derived(
    speaker.preparing || tts.making > 0
      ? "Getting the voice ready"
      : speaker.paused
        ? "Paused"
        : speaker.speakingId !== null
          ? `Reading aloud${speaker.position.of > 1 ? ` · ${speaker.position.at} of ${speaker.position.of}` : ""}`
          : "Ready. Starts only when it reads",
  );

  const count = $derived((chat ? 1 : 0) + (dictation ? 1 : 0) + (making ? 1 : 0) + (voiceBusy ? 1 : 0));
  const busy = $derived(
    chat?.loading || dictation?.loading || speaker.preparing || (making && images.run?.phase !== "making") || false,
  );

  const label = $derived(
    count === 0
      ? "Nothing loaded"
      : count === 1
        ? (chat?.name ?? dictation?.name ?? (voiceBusy ? "Reading aloud" : "Making an image"))
        : `${count} in use`,
  );

  // A bar scaled so "excellent" sits near the end: 150 tokens per second, 40x real time.
  const bar = (value: number, max: number) => `${Math.min(100, Math.max(4, (value / max) * 100))}%`;
</script>

{#snippet tile(Icon: Component<{ size?: number | string }>, pulsing: boolean, off = false)}
  <span class="relative flex size-9 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground">
    <Icon size={18} />
    <span
      class="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-popover {off ? 'bg-muted-foreground/40' : pulsing ? 'animate-pulse bg-warning' : 'bg-success'}"
    ></span>
  </span>
{/snippet}

<Popover>
  <PopoverTrigger>
    {#snippet child({ props })}
      <Button {...props} variant="ghost" size="sm" class="max-w-56 gap-2 text-muted-foreground" aria-label="Models in memory">
        <span
          class="size-2 shrink-0 rounded-full transition-colors duration-300 {count === 0 ? 'bg-muted-foreground/40' : busy ? 'animate-pulse bg-warning' : 'bg-success'}"
        ></span>
        <span class="truncate">{label}</span>
      </Button>
    {/snippet}
  </PopoverTrigger>
  <PopoverContent align="end" class="w-[min(30rem,calc(100vw-2rem))] p-2">
    <div class="flex flex-col gap-1">
      <div class="flex items-center justify-between px-2 pt-1 pb-1.5">
        <p class="text-sm font-medium">In memory</p>
        <span class="text-sm text-muted-foreground tabular-nums">{count === 0 ? "Free" : `${count} in use`}</span>
      </div>

      {#if count === 0 && !chatIdle && !speechIdleName && !voiceReady}
        <div class="flex flex-col items-center gap-2 rounded-xl bg-muted/50 px-4 py-6 text-center motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
          <PowerIcon size={22} class="text-muted-foreground" />
          <p class="text-sm text-pretty">Nothing is loaded, so the memory is free.</p>
          <p class="text-sm text-muted-foreground text-pretty">Models load when you use them and unload when idle.</p>
        </div>
      {/if}

      {#if chat}
        <div class="rounded-xl p-2 transition-colors hover:bg-muted/50 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1 motion-safe:duration-300">
          <div class="flex items-center gap-3">
            {@render tile(ChatLineIcon, chat.loading)}
            <div class="min-w-24 flex-1">
              <p class="truncate text-sm font-medium">{chat.name}</p>
              <p class="text-sm text-muted-foreground">Chat · {chat.loading ? "Loading" : "Ready"}</p>
            </div>
            {#if !chat.loading}
              <Button size="sm" variant="ghost" class="gap-1.5" disabled={llm.testingSpeed} onclick={() => llm.testSpeed()}>
                <SpeedometerLowIcon size={16} />{chatSpeed === null ? "Test" : "Retest"}
              </Button>
            {/if}
            <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => llm.stop()}>
              <PowerIcon size={16} />Unload
            </Button>
          </div>
          {#if llm.testingSpeed}
            <div class="mt-2 pl-12 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-200">
              <Progress indeterminate size="sm" label="Measuring speed" />
              <p class="mt-1 text-sm text-muted-foreground">Measuring speed</p>
            </div>
          {:else if chatSpeed !== null}
            {@const rating = rateSpeed(chatSpeed)}
            <div class="mt-2 pl-12 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
              <div class="h-1.5 overflow-hidden rounded-full bg-muted">
                <div class="h-full rounded-full transition-[width] duration-500 ease-out {fill[rating]}" style:width={bar(chatSpeed, 150)}></div>
              </div>
              <p class="mt-1.5 flex items-center gap-2 text-sm tabular-nums">
                {chatSpeed.toFixed(0)} tokens per second
                <Badge size="sm" variant={tone[rating]}>{speedLabel[rating]}</Badge>
              </p>
            </div>
          {/if}
        </div>
      {/if}

      {#if dictation}
        <div class="rounded-xl p-2 transition-colors hover:bg-muted/50 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1 motion-safe:duration-300">
          <div class="flex items-center gap-3">
            {@render tile(Microphone3Icon, dictation.loading)}
            <div class="min-w-24 flex-1">
              <p class="truncate text-sm font-medium">{dictation.name}</p>
              <p class="text-sm text-muted-foreground">Dictation · {dictation.loading ? "Loading" : "Ready"}</p>
            </div>
            {#if !dictation.loading}
              <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => speech.unload()}>
                <PowerIcon size={16} />Unload
              </Button>
            {/if}
          </div>
          {#if !dictation.loading}
            {#if dictationSpeed !== null}
              {@const rating = rateRealtime(dictationSpeed)}
              <div class="mt-2 pl-12 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
                <div class="h-1.5 overflow-hidden rounded-full bg-muted">
                  <div class="h-full rounded-full transition-[width] duration-500 ease-out {fill[rating]}" style:width={bar(dictationSpeed, 40)}></div>
                </div>
                <p class="mt-1.5 flex items-center gap-2 text-sm tabular-nums">
                  {dictationSpeed.toFixed(0)}x faster than speaking
                  <Badge size="sm" variant={tone[rating]}>{speedLabel[rating]}</Badge>
                </p>
              </div>
            {:else}
              <p class="mt-2 pl-12 text-sm text-muted-foreground">Dictate once and its speed shows here.</p>
            {/if}
          {/if}
        </div>
      {/if}

      {#if chatIdle}
        <div class="flex items-center gap-3 rounded-xl p-2 transition-colors hover:bg-muted/50 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
          {@render tile(ChatLineIcon, false, true)}
          <div class="min-w-24 flex-1">
            <p class="truncate text-sm font-medium">Chat model</p>
            <p class="text-sm text-muted-foreground">Not loaded. It loads when you send a message.</p>
          </div>
          <Button size="sm" variant="secondary" class="gap-1.5" onclick={() => llm.ensureLoaded()}>
            <PlayIcon size={16} />Load
          </Button>
        </div>
      {/if}

      {#if speechIdleName}
        <div class="flex items-center gap-3 rounded-xl p-2 transition-colors hover:bg-muted/50 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
          {@render tile(Microphone3Icon, false, true)}
          <div class="min-w-24 flex-1">
            <p class="truncate text-sm font-medium">{speechIdleName}</p>
            <p class="text-sm text-muted-foreground">Dictation · Not loaded. It loads when you dictate.</p>
          </div>
          <Button size="sm" variant="secondary" class="gap-1.5" onclick={() => speech.use(speech.current ?? "")}>
            <PlayIcon size={16} />Load
          </Button>
        </div>
      {/if}

      {#if voiceReady}
        <div class="rounded-xl p-2 transition-colors hover:bg-muted/50 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1 motion-safe:duration-300">
          <div class="flex items-center gap-3">
            {@render tile(SoundwaveIcon, speaker.preparing || tts.making > 0, !voiceBusy)}
            <div class="min-w-24 flex-1">
              <p class="truncate text-sm font-medium">{tts.modelName} · {tts.voiceName}</p>
              <p class="text-sm text-muted-foreground">Voice · {voiceState}</p>
            </div>
            {#if speaker.speakingId !== null && !speaker.preparing}
              <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => (speaker.paused ? speaker.resume() : speaker.pause())}>
                {#if speaker.paused}<PlayIcon size={16} />Resume{:else}<PauseIcon size={16} />Pause{/if}
              </Button>
              <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => speaker.stop()}>
                <StopCircleIcon size={16} />Stop
              </Button>
            {/if}
          </div>
          <Collapsible class="mt-1 pl-12">
            <CollapsibleTrigger class="flex items-center gap-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground">
              <Tuning2Icon size={14} />Voice and speed
            </CollapsibleTrigger>
            <CollapsibleContent>
              <div class="pt-3 pb-1"><VoicePicker /></div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      {/if}

      {#if images.run}
        <div class="rounded-xl p-2 transition-colors hover:bg-muted/50 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1 motion-safe:duration-300">
          <div class="flex items-center gap-3">
            {@render tile(GalleryIcon, images.run.phase !== "making")}
            <div class="min-w-24 flex-1">
              <p class="truncate text-sm font-medium">{images.model?.name ?? "Image model"}</p>
              <p class="text-sm text-muted-foreground tabular-nums">
                Images ·
                {images.run.phase === "queued"
                  ? "Waiting for chat"
                  : images.run.phase === "starting"
                    ? "Getting ready"
                    : `Step ${images.run.step} of ${images.run.total}`}
              </p>
            </div>
            <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => images.cancel()}>
              <StopCircleIcon size={16} />Stop
            </Button>
          </div>
          {#if images.run.total > 0}
            <div class="mt-2 pl-12">
              <Progress value={(images.run.step / images.run.total) * 100} size="sm" label="Image progress" />
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </PopoverContent>
</Popover>
