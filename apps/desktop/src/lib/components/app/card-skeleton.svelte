<script lang="ts">
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";

  // Placeholder for a model or engine card while the catalog loads.
  let { count = 2 }: { count?: number } = $props();
</script>

<ul class="flex flex-col gap-3" role="status" aria-label="Loading">
  {#each Array.from({ length: count }, (_, i) => i) as i (i)}
    <li class="flex items-start justify-between gap-4 rounded-2xl bg-card p-4">
      <div class="flex min-w-0 flex-1 flex-col gap-2.5">
        <Skeleton class="h-4 w-36" />
        <Skeleton class="h-3.5 w-full max-w-md" />
        <Skeleton class="h-3.5 w-44" />
      </div>
      <Skeleton class="h-8 w-24 rounded-lg" />
    </li>
  {/each}
</ul>
