<script lang="ts">
  import type { Component, Snippet } from "svelte";

  // Decision (product): every section says what it is for in one line, so a long page can be
  // scanned and each setting has context before the controls.
  let {
    title,
    description,
    icon: Icon,
    id,
    children,
  }: {
    title: string;
    description?: string;
    icon?: Component<{ size?: number | string; class?: string }>;
    id?: string;
    children: Snippet;
  } = $props();
</script>

<section {id} class="flex scroll-mt-6 flex-col gap-3">
  <header class="px-1">
    <h2 class="flex items-center gap-2 text-lg">
      {#if Icon}<Icon size={18} class="text-muted-foreground" />{/if}
      {title}
    </h2>
    {#if description}<p class="mt-0.5 text-sm text-muted-foreground text-pretty">{description}</p>{/if}
  </header>
  <div class="rounded-2xl bg-card px-4">{@render children()}</div>
</section>
