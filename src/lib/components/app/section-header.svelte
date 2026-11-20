<script lang="ts">
  import type { Component } from "svelte";

  // Decision (product): every section says what it is for in one line, so a long page can be
  // scanned and each control has context before it.
  let {
    title,
    description,
    icon: Icon,
  }: {
    title: string;
    description?: string;
    icon?: Component<{ size?: number | string; class?: string }>;
  } = $props();
</script>

<header class="px-1">
  <h2 class="flex items-center gap-2 text-lg">
    {#if Icon}<Icon size={18} class="text-muted-foreground" />{/if}
    {title}
  </h2>
  {#if description}<p class="mt-0.5 text-sm text-muted-foreground text-pretty">{description}</p>{/if}
</header>
