<script lang="ts">
  import { AltArrowRightIcon, LockKeyholeIcon, LockKeyholeUnlockedIcon } from "@solar-icons/svelte/linear";
  import type { Artifact } from "../../../bindings";
  import type { CategoryId } from "#lib/mention.js";
  import KindIcon from "#lib/components/app/kind-icon.svelte";

  // Suggestions for an @ mention: first the groups, then the things in one. It is drawn above the
  // composer and driven from the keyboard by the composer, so the text box keeps focus.
  export type MentionRow =
    | { type: "group"; id: CategoryId; label: string }
    | { type: "item"; item: Artifact };

  let {
    rows,
    active,
    onpick,
    heading,
    empty,
  }: {
    rows: MentionRow[];
    active: number;
    onpick: (row: MentionRow) => void;
    /** The group being browsed, shown above its items. */
    heading?: string;
    empty: string;
  } = $props();
</script>

<div
  class="absolute right-2 bottom-full left-2 z-30 mb-2 overflow-hidden rounded-2xl bg-popover p-1 shadow-(--overlay-shadow) motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-150"
  role="listbox"
  aria-label={heading ?? "Mention"}
>
  {#if heading}
    <p class="px-3 pt-1.5 pb-1 text-xs font-medium tracking-wide text-muted-foreground uppercase">{heading}</p>
  {/if}
  {#if rows.length === 0}
    <p class="px-3 py-2 text-sm text-muted-foreground">{empty}</p>
  {:else}
    {#each rows as row, i (row.type === "group" ? `g:${row.id}` : row.item.id)}
      <button
        type="button"
        role="option"
        aria-selected={i === active}
        class="flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left transition-colors {i === active ? 'bg-accent' : 'hover:bg-muted'}"
        onmousedown={(e) => {
          e.preventDefault();
          onpick(row);
        }}
      >
        {#if row.type === "group"}
          <span class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <KindIcon kind={row.id} />
          </span>
          <span class="min-w-0 flex-1 text-sm font-medium">{row.label}</span>
          <AltArrowRightIcon size={14} class="shrink-0 text-muted-foreground" />
        {:else}
          <span class="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
            <KindIcon kind={row.item.kind} />
          </span>
          <span class="min-w-0 flex-1">
            <span class="block truncate text-sm font-medium">{row.item.title}</span>
            <span class="block truncate text-sm text-muted-foreground">{row.item.preview}</span>
          </span>
          {#if row.item.kind !== "file"}
            {#if row.item.privacy === "shareable"}
              <LockKeyholeUnlockedIcon size={14} class="shrink-0 text-warning-strong" />
            {:else}
              <LockKeyholeIcon size={14} class="shrink-0 text-muted-foreground" />
            {/if}
          {/if}
        {/if}
      </button>
    {/each}
  {/if}
</div>
