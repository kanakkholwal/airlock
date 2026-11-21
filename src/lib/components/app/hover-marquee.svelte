<script lang="ts">
  import { marqueeSeconds } from "#lib/marquee.js";

  // Decision (product): one way to show text that does not fit. It ends in an ellipsis at rest
  // and, on hover or keyboard focus, slides right to left so the rest can be read, then slides
  // back. Used wherever space is tight: chat titles, the sidebar, the header.
  let { text, class: classProp = "" }: { text: string; class?: string } = $props();

  let box = $state<HTMLSpanElement>();
  let inner = $state<HTMLSpanElement>();
  let shift = $state(0);
  let seconds = $state(0);
  let active = $state(false);

  function start() {
    if (!box || !inner) return;
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const overflow = Math.ceil(inner.scrollWidth - box.clientWidth);
    if (overflow <= 1) return;
    seconds = marqueeSeconds(overflow);
    active = true;
    // The next frame lets the inline-block layout settle before the slide begins.
    requestAnimationFrame(() => (shift = -overflow));
  }

  function stop() {
    seconds = 0.25;
    shift = 0;
    active = false;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  bind:this={box}
  class="block min-w-0 overflow-hidden whitespace-nowrap {active ? 'text-clip' : 'text-ellipsis'} {classProp}"
  onpointerenter={start}
  onpointerleave={stop}
  onfocusin={start}
  onfocusout={stop}
>
  <span
    bind:this={inner}
    class={active ? "inline-block will-change-transform" : ""}
    style:transform={active ? `translateX(${shift}px)` : undefined}
    style:transition={active ? `transform ${seconds}s linear` : undefined}
  >{text}</span>
</span>
