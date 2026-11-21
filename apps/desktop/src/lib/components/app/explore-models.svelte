<script lang="ts">
  import { onMount } from "svelte";
  import { ArrowRightUpIcon, DangerTriangleIcon, GlobalIcon, MagnifierIcon } from "@solar-icons/svelte/linear";
  import { explore, type ExploreSort, type ExploreTask } from "#lib/explore.svelte.js";
  import { engineSupport, formatParams } from "#lib/hf-compat.js";
  import { inTauri } from "#lib/env.js";
  import HfModelSheet from "#lib/components/app/hf-model-sheet.svelte";
  import HfStats from "#lib/components/app/hf-stats.svelte";
  import PublisherLink from "#lib/components/app/publisher-link.svelte";
  import { hfUrl, modelNameOf, publisherOf } from "#lib/publisher.js";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import StateView from "#lib/components/app/state-view.svelte";
  import { HoverMarquee } from "#lib/components/text/hover-marquee/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Input } from "#lib/components/ui/input/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";
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

</script>

{#if !inTauri()}
  <StateView
    icon={GlobalIcon}
    title="Open the Airlock app to explore models"
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
        {#if explore.searching}
          <Spinner size="sm" label="Searching" class="absolute end-3 top-1/2 -translate-y-1/2" />
        {/if}
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
      GGUF models from Hugging Face. Check each license.
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
          <li class="flex items-start gap-3.5 rounded-2xl bg-card p-4">
            <div class="flex flex-1 flex-col gap-2.5">
              <Skeleton class="h-4 w-56" />
              <Skeleton class="h-3.5 w-80 max-w-full" />
            </div>
            <Skeleton class="h-8 w-16 rounded-lg" />
          </li>
        {/each}
      </ul>
    {:else if explore.searched && explore.results.length === 0}
      <StateView
        icon={MagnifierIcon}
        title="No models match" description="Try another search."
        variant="card"
      />
    {:else}
      <ul class="flex flex-col gap-2 {explore.searching ? 'opacity-60 transition-opacity' : ''}">
        {#each explore.results as m (m.id)}
          {@const support = engineSupport(m.architecture)}
          {@const params = formatParams(m.parameters)}
          <li class="group flex items-start gap-3.5 rounded-2xl bg-card p-4 transition-colors duration-150 hover:bg-muted/50">
            <div class="min-w-0 flex-1">
              <div class="flex items-baseline gap-2">
                <span class="min-w-0 text-sm font-semibold"><HoverMarquee text={modelNameOf(m.id)} /></span>
              </div>
              <PublisherLink name={publisherOf(m.id)} />
              <div class="mt-2 flex flex-wrap items-center gap-1.5">
                <Badge size="sm" variant={support === "likely" ? "success" : "warning"}>
                  {support === "likely" ? "Should run" : "Unverified"}
                </Badge>
                {#if params}<Badge size="sm" variant="secondary">{params}</Badge>{/if}
                {#if m.architecture}<Badge size="sm" variant="secondary">{m.architecture}</Badge>{/if}
                {#if m.vision}<Badge size="sm" variant="info">Vision</Badge>{/if}
                {#if m.license}<Badge size="sm" variant="secondary">{m.license}</Badge>{/if}
              </div>
              <div class="mt-2.5"><HfStats downloads={m.downloads} likes={m.likes} updated={m.last_modified} /></div>
            </div>
            <div class="flex shrink-0 items-center gap-1">
              <Button size="icon-sm" variant="ghost" aria-label={`Open ${m.id} on Hugging Face`} title="View on Hugging Face" onclick={() => openUrl(hfUrl(m.id))}>
                <ArrowRightUpIcon size={16} />
              </Button>
              <Button size="sm" variant="secondary" onclick={() => explore.open(m)}>Get</Button>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<HfModelSheet />
