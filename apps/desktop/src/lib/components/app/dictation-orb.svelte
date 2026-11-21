<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { commands } from "../../../bindings";
  import { inTauri } from "#lib/env.js";
  import FluidOrb from "#lib/components/app/fluid-orb.svelte";

  // Decision (product): the orb is the page's sign of life. It rests while idle and swells
  // with your voice while you dictate, so you can see the microphone hears you before you
  // look at any text. The backend sends levels only while this is on screen.
  let { hint }: { hint: string } = $props();

  let level = $state(0);
  let lastAt = $state(0);
  let now = $state(Date.now());

  const listening = $derived(now - lastAt < 700);

  onMount(() => {
    if (!inTauri()) return;
    void commands.watchMicLevels(true);
    const stop = listen<number[]>("mic-level", (e) => {
      const bands = e.payload;
      const avg = bands.length ? bands.reduce((a, b) => a + b, 0) / bands.length : 0;
      level = Math.min(1, Math.pow(avg, 0.6) * 1.8);
      lastAt = Date.now();
    });
    // The level stops arriving when recording ends, so the clock decides when it is quiet.
    const tick = setInterval(() => {
      now = Date.now();
      if (now - lastAt >= 700) level = 0;
    }, 250);
    return () => {
      clearInterval(tick);
      void commands.watchMicLevels(false);
      void stop.then((u) => u());
    };
  });
</script>

<div class="flex flex-col items-center gap-2 py-2">
  <FluidOrb
    size={140}
    activity={level}
    label={listening ? "Listening to you" : "Waiting for you to speak"}
  />
  <p class="text-sm text-muted-foreground" aria-live="polite">{listening ? "Listening" : hint}</p>
</div>
