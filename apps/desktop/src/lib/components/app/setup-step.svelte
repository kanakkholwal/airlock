<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import { AltArrowDownIcon, CheckCircleIcon } from "@solar-icons/svelte/linear";

  // Decision (product): setup is a short checklist. A finished step shrinks to one line with its
  // current value, an unfinished one opens by itself, and either can be toggled by hand.
  let {
    title,
    summary,
    done,
    icon: Icon,
    children,
  }: { title: string; summary: string; done: boolean; icon: Component<{ size?: number }>; children: Snippet } = $props();

  let manual = $state<boolean | null>(null);
  const open = $derived(manual ?? !done);
</script>

<section class="rounded-2xl bg-card">
  <button
    type="button"
    class="flex w-full items-center gap-3 rounded-2xl p-4 text-left transition-colors hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none"
    aria-expanded={open}
    onclick={() => (manual = !open)}
  >
    <span
      class="flex size-8 shrink-0 items-center justify-center rounded-lg transition-colors duration-300 {done ? 'bg-success/15 text-success-strong' : 'bg-muted text-muted-foreground'}"
    >
      {#if done}<CheckCircleIcon size={18} />{:else}<Icon size={18} />{/if}
    </span>
    <span class="min-w-0 flex-1">
      <span class="block text-sm font-medium">{title}</span>
      <span class="block truncate text-xs text-muted-foreground">{summary}</span>
    </span>
    <AltArrowDownIcon size={16} class="shrink-0 text-muted-foreground transition-transform duration-200 {open ? 'rotate-180' : ''}" />
  </button>
  <div class="grid transition-[grid-template-rows] duration-250 ease-out motion-reduce:transition-none {open ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'}">
    <div class="overflow-hidden" inert={!open}>
      <div class="px-4 pb-4">{@render children()}</div>
    </div>
  </div>
</section>
