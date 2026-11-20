<script lang="ts">
  import type { Snippet } from "svelte";
  import { ScrollArea } from "#lib/components/ui/scroll-area/index.js";

  // Pages that scroll as a whole sit in the Baby UI Scroll Area; the chat page does not,
  // because its transcript is its own scroll region.
  let { children }: { children: Snippet } = $props();
</script>

<ScrollArea maxHeight="100%" scrollbar="hover" class="h-full">
  {@render children()}
</ScrollArea>
