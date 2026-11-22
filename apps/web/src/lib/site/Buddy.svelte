<script lang="ts">
  import Mascot, { type Kind, type Tone } from "./Mascot.svelte";

  // A character that floats in place, a little differently each time: its own loop length, its own
  // reach, its own tilt. Point at one and it squishes. Position it with `class`. With reduced
  // motion it stays put.
  let {
    kind,
    tone,
    tilt = -6,
    delay = 0,
    duration = 5200,
    amplitude = 12,
    class: className = "",
  }: { kind: Kind; tone?: Tone; tilt?: number; delay?: number; duration?: number; amplitude?: number; class?: string } = $props();
</script>

<span
  class="buddy block {className}"
  style="--tilt: {tilt}deg; --amp: {amplitude}px; animation-duration: {duration}ms; animation-delay: {delay}ms"
  aria-hidden="true"
>
  <Mascot {kind} {tone} class="size-full" />
</span>
