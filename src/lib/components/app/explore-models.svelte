<script lang="ts">
  import { onMount } from "svelte";
  import { DangerTriangleIcon, GlobalIcon, MagnifierIcon } from "@solar-icons/svelte/linear";
  import { explore, type ExploreSort, type ExploreTask } from "#lib/explore.svelte.js";
  import { engineSupport, formatCount, formatParams } from "#lib/hf-compat.js";
  import { inTauri } from "#lib/env.js";
  import HfModelDialog from "#lib/components/app/hf-model-dialog.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Input } from "#lib/components/ui/input/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";

  // Decision (product): Explore is for adding a model that is not in the short recommended list.
  // It only searches GGUF models, since that is what the engine runs, and it says so in each
  // result with a "should run" or "unverified" tag. Searching contacts huggingface.co, and only
  // while this tab is open.
  const sorts: { value: ExploreSort; label: string }[] = [
    { value: "trendingScore", label: "Trending" },
    { value: "downloads", label: "Most downloaded" },
    { value: "likes", label: "Most liked" },
    { value: "lastModified", label: "Recently updated" },
  ];

  onMount(() => {
    if (!explore.searched) void explore.search();
  });

  const updated = (iso: string | null) =>
    iso ? new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(new Date(iso)) : "";
</script>

{#if !inTauri()}
  <StateView
    icon={GlobalIcon}
    title="Open the Airlock app to explore models"
    description="Searching and downloading is done by the desktop app."
    variant="card"
  />
{:else}
  <div class="flex flex-col gap-4">
    <div class="flex flex-wrap items-center gap-2">
      <div class="relative min-w-52 flex-1">
        <MagnifierIcon size={16} class="pointer-events-none absolute start-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
        <Input
          class="ps-9"
          placeholder="Search models, for example qwen or gemma"
          aria-label="Search models"
          bind:value={explore.query}
          oninput={() => explore.schedule()}
        />
      </div>
      <ToggleGroup
        label="Model type"
        value={explore.task}
        onValueChange={(v) => {
          if (typeof v === "string" && v) {
            explore.task = v as ExploreTask;
            void explore.search();
          }
        }}
      >
        <ToggleGroupItem value="chat">Chat</ToggleGroupItem>
        <ToggleGroupItem value="vision">Vision</ToggleGroupItem>
      </ToggleGroup>
      <Select
        items={sorts}
        bind:value={
          () => explore.sort,
          (v) => {
            explore.sort = v as ExploreSort;
            void explore.search();
          }
        }
      >
        <SelectTrigger class="w-48">
          <span class="truncate">{sorts.find((s) => s.value === explore.sort)?.label}</span>
        </SelectTrigger>
        <SelectContent align="end">
          {#each sorts as s (s.value)}
            <SelectItem value={s.value}>{s.label}</SelectItem>
          {/each}
        </SelectContent>
      </Select>
    </div>

    <p class="px-1 text-sm text-muted-foreground text-pretty">
      GGUF models from huggingface.co. "Should run" means the engine is known to handle that model type; always
      check the license before you use a model.
    </p>

    {#if explore.error}
      <StateView
        icon={DangerTriangleIcon}
        tone="warning"
        title="Could not reach Hugging Face"
        description={explore.error}
        variant="card"
      >
        {#snippet actions()}<Button variant="secondary" onclick={() => explore.search()}>Try again</Button>{/snippet}
      </StateView>
    {:else if explore.searching && explore.results.length === 0}
      <ul class="flex flex-col gap-3" role="status" aria-label="Searching">
        {#each [0, 1, 2, 3] as i (i)}
          <li class="flex items-start justify-between gap-4 rounded-2xl bg-card p-4">
            <div class="flex flex-1 flex-col gap-2.5">
              <Skeleton class="h-4 w-56" />
              <Skeleton class="h-3.5 w-80 max-w-full" />
            </div>
            <Skeleton class="h-8 w-20 rounded-lg" />
          </li>
        {/each}
      </ul>
    {:else if explore.searched && explore.results.length === 0}
      <StateView
        icon={MagnifierIcon}
        title="No models match"
        description="Try a shorter search, or switch between Chat and Vision."
        variant="card"
      />
    {:else}
      <ul class="flex flex-col gap-3 {explore.searching ? 'opacity-60 transition-opacity' : ''}">
        {#each explore.results as m (m.id)}
          {@const support = engineSupport(m.architecture)}
          {@const params = formatParams(m.parameters)}
          <li class="flex items-start justify-between gap-4 rounded-2xl bg-card p-4">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <span class="font-[450]">{m.id.split("/")[1]}</span>
                <span class="text-sm text-muted-foreground">{m.id.split("/")[0]}</span>
              </div>
              <div class="mt-2 flex flex-wrap items-center gap-1.5">
                <Badge size="sm" variant={support === "likely" ? "success" : "warning"}>
                  {support === "likely" ? "Should run" : "Unverified"}
                </Badge>
                {#if m.architecture}<Badge size="sm" variant="secondary">{m.architecture}</Badge>{/if}
                {#if params}<Badge size="sm" variant="secondary">{params} parameters</Badge>{/if}
                {#if m.vision}<Badge size="sm" variant="info">Vision</Badge>{/if}
                {#if m.license}<Badge size="sm" variant="secondary">{m.license}</Badge>{/if}
              </div>
              <p class="mt-2 text-sm text-muted-foreground tabular-nums">
                {formatCount(m.downloads)} downloads · {formatCount(m.likes)} likes{m.last_modified ? ` · updated ${updated(m.last_modified)}` : ""}
              </p>
            </div>
            <Button size="sm" variant="secondary" onclick={() => explore.open(m)}>Get</Button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<HfModelDialog />
