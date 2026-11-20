<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    description,
    children,
  }: { title: string; description?: string; children: Snippet } = $props();
</script>

<!-- Decision (product): rows separate by a hairline only because they form a list. -->
<div class="flex items-center justify-between gap-6 py-3 not-first:border-t not-first:border-border/60">
  <div class="min-w-0">
    <div class="font-medium">{title}</div>
    {#if description}
      <div class="text-sm text-muted-foreground text-pretty">{description}</div>
    {/if}
  </div>
  <div class="shrink-0">{@render children()}</div>
</div>
