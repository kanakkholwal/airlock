<script lang="ts">
  import { ClockCircleIcon, DownloadMinimalisticIcon, HeartIcon } from "@solar-icons/svelte/linear";
  import { formatCount } from "#lib/hf-compat.js";

  let { downloads, likes, updated }: { downloads: number; likes: number; updated?: string | null } = $props();
  const when = $derived(updated ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(new Date(updated)) : "");
</script>

<span class="flex flex-wrap items-center gap-x-3.5 gap-y-1 text-xs text-muted-foreground tabular-nums">
  <span class="flex items-center gap-1" title="Downloads"><DownloadMinimalisticIcon size={14} />{formatCount(downloads)}</span>
  <span class="flex items-center gap-1" title="Likes"><HeartIcon size={14} />{formatCount(likes)}</span>
  {#if when}<span class="flex items-center gap-1" title="Last updated"><ClockCircleIcon size={14} />{when}</span>{/if}
</span>
