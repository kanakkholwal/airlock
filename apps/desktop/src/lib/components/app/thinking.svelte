<script lang="ts">
  import { AltArrowRightIcon } from "@solar-icons/svelte/linear";
  import { onMount, tick } from "svelte";

  // Decision (product): thinking reads as a quiet line, not a box. While the model thinks it
  // shimmers with a running timer and shows the live text; once the answer starts it folds to
  // "Thought for 4s". A hand toggle always wins over the automatic open and close.
  let {
    text = "",
    thinking,
    seconds = 0,
  }: { text?: string; thinking: boolean; seconds?: number } = $props();

  let manual = $state<boolean | null>(null);
  const open = $derived(manual ?? thinking);
  const hasText = $derived(text.trim().length > 0);

  let elapsed = $state(0);
  let scroller = $state<HTMLDivElement>();

  onMount(() => {
    const started = Date.now();
    const id = setInterval(() => {
      if (thinking) elapsed = Math.floor((Date.now() - started) / 1000);
    }, 500);
    return () => clearInterval(id);
  });

  const shown = $derived(thinking ? elapsed : seconds || elapsed);
  const label = $derived(thinking ? "Thinking" : shown > 0 ? `Thought for ${shown}s` : "Thought");

  // Follow the newest line while it streams, unless the reader scrolled up.
  $effect(() => {
    text;
    if (!thinking || !scroller) return;
    const el = scroller;
    const nearEnd = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    if (nearEnd) void tick().then(() => (el.scrollTop = el.scrollHeight));
  });
</script>

<div class="flex flex-col" data-slot="thinking" data-state={thinking ? "thinking" : "done"}>
  <button
    type="button"
    class="group flex w-fit items-center gap-2 rounded-lg py-1 pe-2 text-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none disabled:cursor-default"
    aria-expanded={open}
    disabled={!hasText}
    onclick={() => (manual = !open)}
  >
    {#if thinking}
      <span class="relative flex size-2.5 items-center justify-center" aria-hidden="true">
        <span class="absolute inline-flex size-full rounded-full bg-primary/40 motion-safe:animate-ping"></span>
        <span class="relative size-1.5 rounded-full bg-primary"></span>
      </span>
    {:else if hasText}
      <AltArrowRightIcon size={14} class="transition-transform duration-200 {open ? 'rotate-90' : ''}" />
    {/if}
    <span class={thinking ? "shimmer font-medium" : ""}>{label}</span>
    {#if thinking && shown > 0}<span class="tabular-nums opacity-70">{shown}s</span>{/if}
  </button>

  <div class="grid transition-[grid-template-rows] duration-300 ease-out motion-reduce:transition-none {open && hasText ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'}">
    <div class="overflow-hidden" inert={!(open && hasText)}>
      <div
        bind:this={scroller}
        class="mt-1 ms-1 max-h-56 overflow-y-auto border-s-2 border-border/70 ps-4 text-sm leading-relaxed whitespace-pre-wrap text-muted-foreground [scrollbar-width:thin] {thinking ? 'mask-[linear-gradient(to_bottom,transparent,#000_12px)]' : ''}"
      >
        {text}
      </div>
    </div>
  </div>
</div>

<style>
  .shimmer {
    background-image: linear-gradient(90deg, var(--muted-foreground) 35%, var(--foreground) 50%, var(--muted-foreground) 65%);
    background-size: 250% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    color: transparent;
    animation: thinking-shimmer 1.8s linear infinite;
  }
  @keyframes thinking-shimmer {
    from { background-position: 120% 0; }
    to { background-position: -120% 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .shimmer { animation: none; color: var(--muted-foreground); background-image: none; }
  }
</style>
