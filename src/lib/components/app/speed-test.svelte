<script lang="ts">
  import { SpeedometerLowIcon } from "@solar-icons/svelte/linear";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuLabel,
    DropdownMenuTrigger,
  } from "#lib/components/ui/dropdown-menu/index.js";
  import { rateRealtime, rateSpeed, speedLabel, type SpeedRating, type SpeedResult, type SpeedTarget } from "#lib/speed.js";

  let {
    run,
    available = ["chat", "dictation", "both"],
  }: {
    run: ((target: SpeedTarget) => Promise<SpeedResult[]>) | null;
    available?: SpeedTarget[];
  } = $props();

  const targets: { key: SpeedTarget; label: string }[] = [
    { key: "chat", label: "Chat model" },
    { key: "dictation", label: "Dictation" },
    { key: "both", label: "Both together" },
  ];
  const variant: Record<SpeedRating, "success" | "info" | "warning" | "destructive"> = {
    excellent: "success",
    decent: "info",
    usable: "warning",
    slow: "destructive",
  };

  let running = $state<SpeedTarget | null>(null);
  let results = $state<SpeedResult[]>([]);
  let error = $state<string | null>(null);

  async function start(target: SpeedTarget) {
    if (!run) return;
    running = target;
    error = null;
    try {
      results = await run(target);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      running = null;
    }
  }
</script>

<DropdownMenu>
  <DropdownMenuTrigger>
    {#snippet child({ props })}
      <Button {...props} variant="ghost" size="sm" class="gap-1.5 text-muted-foreground">
        <SpeedometerLowIcon size={16} />
        <span>Speed test</span>
      </Button>
    {/snippet}
  </DropdownMenuTrigger>
  <DropdownMenuContent align="end" class="w-80 p-3">
    <DropdownMenuLabel>Device speed test</DropdownMenuLabel>
    {#if !run}
      <p class="px-2 py-3 text-sm text-muted-foreground text-pretty">
        Needs the local engine. It appears here once a model is installed.
      </p>
    {:else}
      <div class="flex flex-wrap gap-2 px-2 py-2">
        {#each targets.filter((t) => available.includes(t.key)) as t (t.key)}
          <Button size="sm" variant="secondary" disabled={running !== null} onclick={() => start(t.key)}>
            {running === t.key ? "Running" : t.label}
          </Button>
        {/each}
      </div>
      {#if error}
        <p class="px-2 text-sm text-destructive-strong" role="alert">{error}</p>
      {/if}
      <ul class="flex flex-col gap-1 px-2 pt-1">
        {#each results as r (r.target + r.backend)}
          {@const rating = r.unit === "tok/s" ? rateSpeed(r.value) : rateRealtime(r.value)}
          <li class="flex items-center justify-between gap-3 text-sm">
            <span class="min-w-0 truncate">{r.target} on {r.backend}</span>
            <span class="flex shrink-0 items-center gap-2">
              <span class="tabular-nums">{r.value.toFixed(0)} {r.unit}</span>
              <Badge size="sm" variant={variant[rating]}>{speedLabel[rating]}</Badge>
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </DropdownMenuContent>
</DropdownMenu>
