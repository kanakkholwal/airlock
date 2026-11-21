<script lang="ts">
  import type { Component, Snippet } from "svelte";

  let {
    title,
    description,
    icon: Icon,
    children,
  }: { title: string; description?: string; icon?: Component<{ size?: number }>; children: Snippet } = $props();
</script>

<!-- Decision (product): rows separate by a hairline only because they form a list. -->
<div class="-mx-2 flex items-center justify-between gap-6 rounded-xl px-2 py-3.5 transition-colors duration-150 hover:bg-muted/40 not-first:border-t not-first:border-border/60">
  <div class="flex min-w-0 items-center gap-3">
    {#if Icon}
      <span class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground"><Icon size={16} /></span>
    {/if}
    <div class="min-w-0">
      <div class="text-sm font-medium">{title}</div>
      {#if description}
        <div class="text-xs text-muted-foreground text-pretty">{description}</div>
      {/if}
    </div>
  </div>
  <div class="shrink-0">{@render children()}</div>
</div>
