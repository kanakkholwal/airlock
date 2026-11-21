<script lang="ts">
  import { LockKeyholeIcon, LockKeyholeUnlockedIcon } from "@solar-icons/svelte/linear";
  import type { Artifact } from "../../../bindings";
  import KindIcon from "#lib/components/app/kind-icon.svelte";

  // Suggestions for an @ mention. It is drawn above the composer and driven from the keyboard by
  // the composer, so the text box keeps focus the whole time.
  let {
    items,
    active,
    onpick,
  }: { items: Artifact[]; active: number; onpick: (item: Artifact) => void } = $props();
</script>

<div
  class="absolute right-2 bottom-full left-2 z-30 mb-2 overflow-hidden rounded-2xl bg-popover p-1 shadow-(--overlay-shadow) motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-150"
  role="listbox"
  aria-label="Library items"
>
  {#if items.length === 0}
    <p class="px-3 py-2 text-sm text-muted-foreground">Nothing in your library matches.</p>
  {:else}
    {#each items as item, i (item.id)}
      <button
        type="button"
        role="option"
        aria-selected={i === active}
        class="flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left transition-colors {i === active ? 'bg-accent' : 'hover:bg-muted'}"
        onmousedown={(e) => {
          e.preventDefault();
          onpick(item);
        }}
      >
        <span class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
          <KindIcon kind={item.kind} />
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm font-medium">{item.title}</span>
          <span class="block truncate text-sm text-muted-foreground">{item.preview}</span>
        </span>
        {#if item.privacy === "shareable"}
          <LockKeyholeUnlockedIcon size={14} class="shrink-0 text-warning-strong" />
        {:else}
          <LockKeyholeIcon size={14} class="shrink-0 text-muted-foreground" />
        {/if}
      </button>
    {/each}
  {/if}
</div>
