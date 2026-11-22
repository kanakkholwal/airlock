<script lang="ts">
  import { onMount } from "svelte";
  import {
    ImageGeneration,
    ImageGenerationBadge,
    ImageGenerationCaption,
    ImageGenerationField,
    ImageGenerationFrame,
    ImageGenerationMedia,
    ImageGenerationStatus,
    type ImageGenerationPhase,
  } from "#lib/components/agents/image-generation/index.js";

  // A wide layout: the prompt on one side, typed out as it would be in the app, and the picture
  // being made on the other, big enough to see the dots resolve. It loops between making and
  // refining; no finished picture is shown, because none ships with the page.
  const prompt = "A paper boat on a pond at sunrise";
  let typed = $state("");
  let phase = $state<ImageGenerationPhase>("generating");

  onMount(() => {
    let alive = true;
    const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    (async () => {
      while (alive) {
        typed = "";
        phase = "queued";
        for (const ch of prompt) {
          if (!alive) return;
          typed += ch;
          await sleep(reduced ? 0 : 45);
        }
        await sleep(500);
        phase = "generating";
        await sleep(2600);
        phase = "refining";
        await sleep(2400);
      }
    })();
    return () => {
      alive = false;
    };
  });
</script>

<div class="grid items-center gap-8 sm:grid-cols-2 sm:gap-10">
  <div class="flex flex-col gap-3">
    <p class="min-h-16 text-xl leading-snug font-medium text-pretty sm:text-2xl">
      {typed}<span class="ms-0.5 inline-block h-6 w-[2px] translate-y-1 bg-foreground/60 motion-safe:animate-pulse" aria-hidden="true"></span>
    </p>
    <p class="text-sm text-muted-foreground">8 steps · on your graphics card · saved to your library</p>
  </div>

  <div class="mx-auto w-full max-w-sm">
    <ImageGeneration status={phase} variant="fluid">
      <ImageGenerationFrame label={prompt}>
        <ImageGenerationMedia />
        <ImageGenerationField />
        <ImageGenerationBadge>512 × 512</ImageGenerationBadge>
      </ImageGenerationFrame>
      <ImageGenerationCaption><ImageGenerationStatus /></ImageGenerationCaption>
    </ImageGeneration>
  </div>
</div>
