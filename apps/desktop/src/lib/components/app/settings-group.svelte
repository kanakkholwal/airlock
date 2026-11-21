<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import SectionHeader from "#lib/components/app/section-header.svelte";

  let {
    title,
    description,
    icon,
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

<section {id} class="flex scroll-mt-6 flex-col gap-3 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-300">
  <SectionHeader {title} {description} {icon} />
  <div class="rounded-2xl bg-card px-4">{@render children()}</div>
</section>
