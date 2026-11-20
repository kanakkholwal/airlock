<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<section class="flex flex-col gap-2">
  <h2 class="px-1 text-sm font-medium text-muted-foreground">{title}</h2>
  <div class="rounded-2xl bg-card px-4">{@render children()}</div>
</section>
