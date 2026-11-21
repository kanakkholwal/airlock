<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    description,
    children,
  }: { title: string; description?: string; children: Snippet } = $props();
</script>

<!-- Decision (product): rows separate by a hairline only because they form a list. -->
<div class="-mx-2 flex items-center justify-between gap-6 rounded-xl px-2 py-3 transition-colors duration-150 hover:bg-muted/40 not-first:border-t not-first:border-border/60">
  <div class="min-w-0">
    <div class="font-[450]">{title}</div>
    {#if description}
      <div class="text-sm text-muted-foreground text-pretty">{description}</div>
    {/if}
  </div>
  <div class="shrink-0">{@render children()}</div>
</div>
