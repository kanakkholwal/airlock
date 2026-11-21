<script lang="ts">
  import { MenuDotsIcon } from "@solar-icons/svelte/linear";
  import { Button } from "#lib/components/ui/button/index.js";
  import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "#lib/components/ui/dropdown-menu/index.js";

  // The less common actions on a card (test, delete) live here so the card shows one main button.
  let {
    items,
    label = "More actions",
  }: {
    items: { label: string; onselect: () => void; disabled?: boolean }[];
    label?: string;
  } = $props();
</script>

{#if items.length > 0}
  <DropdownMenu>
    <DropdownMenuTrigger>
      {#snippet child({ props })}
        <Button {...props} size="icon-sm" variant="ghost" aria-label={label}><MenuDotsIcon size={18} /></Button>
      {/snippet}
    </DropdownMenuTrigger>
    <DropdownMenuContent align="end">
      {#each items as item (item.label)}
        <DropdownMenuItem disabled={item.disabled} onclick={item.onselect}>{item.label}</DropdownMenuItem>
      {/each}
    </DropdownMenuContent>
  </DropdownMenu>
{/if}
