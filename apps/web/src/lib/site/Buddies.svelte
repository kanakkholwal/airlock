<script lang="ts">
  import Buddy from "./Buddy.svelte";
  import { pick } from "./buddies.js";

  // Places a few characters in a section's margins. `section` seeds which shapes, colours and
  // motion they get, so each section has its own mix instead of the same pair repeated. `spots`
  // are the positions, as classes, and decide how many there are.
  let { section, spots }: { section: string; spots: string[] } = $props();
  const picks = $derived(pick(section, spots.length));
</script>

{#each picks as p, i (i)}
  <Buddy kind={p.kind} tone={p.tone} tilt={p.tilt} delay={p.delay} duration={p.duration} amplitude={p.amplitude} class={spots[i]} />
{/each}
