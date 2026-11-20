<script lang="ts">
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";

  // Same shape as SettingsGroup and SettingRow, so the page does not jump when it loads.
  let { groups = 3, rows = 2 }: { groups?: number; rows?: number } = $props();
</script>

<div class="flex flex-col gap-8" role="status" aria-label="Loading settings">
  {#each Array.from({ length: groups }, (_, i) => i) as g (g)}
    <section class="flex flex-col gap-2">
      <Skeleton class="ms-1 h-4 w-24" />
      <div class="rounded-2xl bg-card px-4">
        {#each Array.from({ length: rows }, (_, i) => i) as r (r)}
          <div class="flex items-center justify-between gap-6 py-3.5">
            <div class="flex flex-col gap-2">
              <Skeleton class="h-4 w-40" />
              <Skeleton class="h-3.5 w-64 max-w-full" />
            </div>
            <Skeleton class="h-6 w-11 rounded-full" />
          </div>
        {/each}
      </div>
    </section>
  {/each}
</div>
