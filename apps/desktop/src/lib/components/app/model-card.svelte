<script lang="ts">
  import type { Component, Snippet } from "svelte";
  import { Card } from "#lib/components/ui/card/index.js";

  // Decision (product): every model, engine and voice on the Models page is one card with the
  // same bones: icon, name and badges, one line of what it is, a quiet facts line, and its
  // actions on the right. A model in use gets a primary-tinted icon, nothing louder. Progress and warnings sit underneath.
  let {
    icon: Icon,
    title,
    description,
    facts = [],
    badges,
    actions,
    specs,
    footer,
    active = false,
  }: {
    icon: Component<{ size?: number | string; class?: string }>;
    title: string;
    description?: string;
    /** Short facts such as a size or a licence, shown in one line with dots between. */
    facts?: (string | null | undefined | false)[];
    badges?: Snippet;
    actions?: Snippet;
    /** Icon chips for what the model needs: disk, memory, context. */
    specs?: Snippet;
    footer?: Snippet;
    active?: boolean;
  } = $props();

  const shown = $derived(facts.filter((f): f is string => !!f));
</script>

<li class="list-none">
  <Card class="gap-3 p-4">
    <div class="flex items-start gap-3.5">
      <span
        class="flex size-10 shrink-0 items-center justify-center rounded-xl transition-colors {active ? 'bg-primary/12 text-primary' : 'bg-muted text-muted-foreground'}"
      >
        <Icon size={20} />
      </span>
      <div class="min-w-0 flex-1">
        <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
          <span class="text-sm font-semibold">{title}</span>
          {@render badges?.()}
        </div>
        {#if description}
          <p class="mt-1 text-sm text-muted-foreground text-pretty">{description}</p>
        {/if}
        {#if specs}
          <div class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground tabular-nums">{@render specs()}</div>
        {/if}
        {#if shown.length > 0}
          <p class="mt-1.5 text-xs text-muted-foreground tabular-nums">{shown.join(" · ")}</p>
        {/if}
      </div>
      {#if actions}
        <div class="flex shrink-0 items-center gap-1.5">{@render actions()}</div>
      {/if}
    </div>
    {#if footer}{@render footer()}{/if}
  </Card>
</li>
