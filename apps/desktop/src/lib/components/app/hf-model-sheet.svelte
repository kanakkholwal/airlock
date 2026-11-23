<script lang="ts">
  import { ArrowRightUpIcon, DangerTriangleIcon, DocumentTextIcon, EyeIcon } from "@solar-icons/svelte/linear";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { toast } from "svelte-sonner";
  import { explore } from "#lib/explore.svelte.js";
  import { engineSupport, fitFor, formatCount, formatParams, memoryNeeded, quantOf, suggestFile, type Fit } from "#lib/hf-compat.js";
  import { hfUrl, modelNameOf, publisherOf } from "#lib/publisher.js";
  import HfStats from "#lib/components/app/hf-stats.svelte";
  import PublisherLink from "#lib/components/app/publisher-link.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { HoverMarquee } from "#lib/components/text/hover-marquee/index.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Drawer, DrawerContent, DrawerDescription, DrawerFooter, DrawerHeader, DrawerTitle } from "#lib/components/ui/drawer/index.js";
  import { Markdown } from "#lib/components/ui/markdown/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";
  import { ScrollArea } from "#lib/components/ui/scroll-area/index.js";
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";
  import { cleanReadme } from "#lib/readme-clean.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";

  // Decision (product): a wide side drawer, not a dialog, so the results stay visible behind it.
  // Files and the model card are two tabs; a side column keeps the facts that decide whether to
  // download. It preselects the file that fits this machine best. Closing is ignored mid-download.
  const repo = $derived(explore.selected);
  const m = $derived(repo?.model);
  const free = $derived(explore.memory?.usable_bytes ?? 0);
  const models = $derived((repo?.files ?? []).filter((f) => !f.projector));
  const projectors = $derived((repo?.files ?? []).filter((f) => f.projector));

  let tab = $state<"files" | "readme">("files");
  let chosen = $state<string | null>(null);
  $effect(() => {
    if (repo && !chosen && models.length > 0) chosen = suggestFile(models, free)?.name ?? null;
    if (!repo) {
      chosen = null;
      tab = "files";
    }
  });
  // The model card loads the first time its tab is opened.
  $effect(() => {
    if (tab === "readme") void explore.loadReadme();
  });

  const file = $derived(models.find((f) => f.name === chosen) ?? null);
  // A vision model needs its projector; the smallest one is enough and is fetched with it.
  const projector = $derived(m?.vision && projectors.length > 0 ? projectors[0] : null);
  const total = $derived((file?.size_bytes ?? 0) + (projector?.size_bytes ?? 0));
  const visionBlocked = $derived(!!m?.vision && !projector && !explore.loadingRepo);
  const params = $derived(formatParams(m?.parameters));
  const support = $derived(engineSupport(m?.architecture));
  const fit = $derived(file ? fitFor(file.size_bytes, free) : null);
  const readme = $derived(cleanReadme(explore.readme ?? ""));

  const gb = (b: number) => (b >= 1_073_741_824 ? `${(b / 1_073_741_824).toFixed(1)} GB` : `${Math.round(b / 1_048_576)} MB`);
  const fitLabel: Record<Fit, { text: string; variant: "success" | "warning" | "destructive" }> = {
    fits: { text: "Fits", variant: "success" },
    tight: { text: "Tight", variant: "warning" },
    "too-big": { text: "Too big", variant: "destructive" },
  };
  const fitNote: Record<Fit, string> = {
    fits: "Should load comfortably.",
    tight: "May load, but other apps could slow down.",
    "too-big": "Not enough free memory. Pick a smaller file.",
  };

  async function get() {
    if (!repo || !file) return;
    const ok = await explore.fetch(repo.model.id, file, projector);
    if (ok) {
      toast(`${modelNameOf(repo.model.id)} added`, { description: "Find it under Chat." });
      explore.close();
    }
  }
</script>

{#snippet fact(label: string, value: string | null | undefined)}
  {#if value}
    <div class="flex items-baseline justify-between gap-3 py-1.5 text-sm">
      <dt class="text-muted-foreground">{label}</dt>
      <dd class="min-w-0 truncate text-end font-medium">{value}</dd>
    </div>
  {/if}
{/snippet}

<Drawer
  direction="right"
  bind:open={
    () => repo !== null,
    (v) => {
      if (!v && !explore.download) explore.close();
    }
  }
>
  <DrawerContent
    variant="framed"
    class="w-full data-[vaul-drawer-direction=right]:w-[68vw]! data-[vaul-drawer-direction=right]:max-w-5xl! data-[vaul-drawer-direction=right]:min-w-[32rem]"
    handle={false}
  >
    {#if m}
      <DrawerHeader class="gap-2 px-1 pb-3 text-start">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <DrawerTitle class="text-lg"><HoverMarquee text={modelNameOf(m.id)} /></DrawerTitle>
            <DrawerDescription class="sr-only">Choose a file to download</DrawerDescription>
            <PublisherLink name={publisherOf(m.id)} />
          </div>
          <Button size="sm" variant="ghost" class="shrink-0 gap-1.5" onclick={() => openUrl(hfUrl(m.id))}>
            <span aria-hidden="true" class="text-base leading-none">&#129303;</span>
            Hugging Face <ArrowRightUpIcon size={14} />
          </Button>
        </div>
        <div class="flex flex-wrap items-center gap-2 pt-2">
          <Badge size="sm" variant={support === "likely" ? "success" : "warning"}>{support === "likely" ? "Should run" : "Unverified"}</Badge>
          {#if params}<Badge size="sm" variant="secondary">{params}</Badge>{/if}
          {#if m.architecture}<Badge size="sm" variant="secondary">{m.architecture}</Badge>{/if}
          {#if m.vision}<Badge size="sm" variant="info"><EyeIcon size={12} />Vision</Badge>{/if}
        </div>
      </DrawerHeader>

      <Tabs bind:value={tab} variant="underline" class="mt-4 mb-2 self-start px-1">
        <TabsList>
          <TabsTrigger value="files">Files</TabsTrigger>
          <TabsTrigger value="readme">README</TabsTrigger>
        </TabsList>
      </Tabs>

      <div class="grid min-h-0 flex-1 gap-6 px-1 pt-3 {tab === 'files' ? 'md:grid-cols-[minmax(0,1fr)_17rem]' : ''}">
        <div class="flex min-h-0 flex-col gap-4">
          {#if tab === "files"}
            {#if explore.loadingRepo}
              <div class="grid grid-cols-2 gap-2 lg:grid-cols-3" role="status" aria-label="Loading files">
                {#each [0, 1, 2, 3, 4, 5] as i (i)}<Skeleton class="h-24 rounded-xl" />{/each}
              </div>
            {:else if explore.repoError}
              <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{explore.repoError}</AlertDescription></Alert>
            {:else if models.length === 0}
              <Alert variant="warning"><DangerTriangleIcon size={18} /><AlertDescription>No single-file GGUF to download here.</AlertDescription></Alert>
            {:else}
              <p class="text-xs text-muted-foreground">Smaller files are faster and use less memory. Larger ones are more accurate.</p>
              <ScrollArea maxHeight="100%" class="-mx-1 min-h-0 flex-1">
                <div class="grid grid-cols-2 gap-3 px-1 py-1 lg:grid-cols-3" role="radiogroup" aria-label="Model file">
                  {#each models as f (f.name)}
                    {@const ft = fitLabel[fitFor(f.size_bytes, free)]}
                    <button
                      type="button"
                      role="radio"
                      aria-checked={chosen === f.name}
                      disabled={explore.download !== null}
                      onclick={() => (chosen = f.name)}
                      class="group flex min-w-0 flex-col gap-3 rounded-xl bg-muted/40 p-4 text-start outline-none transition-[background-color,box-shadow] duration-150 hover:bg-muted/70 focus-visible:ring-2 focus-visible:ring-ring aria-checked:bg-primary/8 aria-checked:ring-1 aria-checked:ring-primary/50 disabled:opacity-60"
                    >
                      <span class="flex items-center justify-between gap-2">
                        <span class="truncate text-sm font-semibold">{quantOf(f.name) ?? "File"}</span>
                        <span
                          aria-hidden="true"
                          class="flex size-4 shrink-0 items-center justify-center rounded-full border border-border transition-colors group-aria-checked:border-primary group-aria-checked:bg-primary"
                        >
                          <span class="size-1.5 rounded-full bg-primary-foreground opacity-0 transition-opacity group-aria-checked:opacity-100"></span>
                        </span>
                      </span>
                      <span class="text-xl leading-none font-semibold tabular-nums">{gb(f.size_bytes)}</span>
                      <span class="flex items-center justify-between gap-2">
                        <Badge size="sm" variant={ft.variant}>{ft.text}</Badge>
                      </span>
                      <HoverMarquee text={f.name} class="block text-xs text-muted-foreground" />
                    </button>
                  {/each}
                </div>
              </ScrollArea>
              {#if projector}
                <p class="text-xs text-muted-foreground">Includes the vision projector ({gb(projector.size_bytes)}).</p>
              {/if}
              {#if visionBlocked}
                <Alert variant="warning"><DangerTriangleIcon size={18} /><AlertDescription>No projector file, so it will read text only.</AlertDescription></Alert>
              {/if}
            {/if}
          {:else}
            <ScrollArea maxHeight="100%" class="min-h-0 flex-1">
              {#if explore.readmeLoading || (explore.readme === null && !explore.readmeError)}
                <div class="flex flex-col gap-3 px-1 pe-4 pt-2" role="status" aria-label="Loading README">
                  <Skeleton class="h-6 w-2/3" />
                  <Skeleton class="h-4 w-full" />
                  <Skeleton class="h-4 w-11/12" />
                  <Skeleton class="h-4 w-4/5" />
                  <Skeleton class="mt-2 h-24 w-full rounded-xl" />
                </div>
              {:else if explore.readmeError}
                <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{explore.readmeError}</AlertDescription></Alert>
              {:else if !readme}
                <StateView icon={DocumentTextIcon} title="No README" description="The publisher has not written one." />
              {:else}
                <!-- Links in a model card open in the browser; they must never navigate the app window. -->
                <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                <div
                  class="px-1 pe-4 pt-2"
                  onclick={(e) => {
                    const link = (e.target as HTMLElement).closest("a");
                    if (!link) return;
                    e.preventDefault();
                    const href = link.getAttribute("href") ?? "";
                    if (/^https?:\/\//i.test(href)) void openUrl(href);
                  }}
                >
                  <Markdown content={readme} />
                </div>
              {/if}
            </ScrollArea>
          {/if}
        </div>

        {#if tab === "files"}
        <aside class="flex min-h-0 flex-col gap-5 overflow-y-auto rounded-2xl bg-muted/40 p-5 md:order-last">
          <div>
            <h3 class="mb-1 text-xs font-semibold text-muted-foreground">About</h3>
            <dl class="divide-y divide-border/50">
              {@render fact("License", m.license)}
              {@render fact("Architecture", m.architecture)}
              {@render fact("Parameters", params)}
              {@render fact("Context", m.context_length ? `${formatCount(m.context_length)} tokens` : null)}
            </dl>
          </div>
          <div>
            <h3 class="mb-1.5 text-xs font-semibold text-muted-foreground">Community</h3>
            <HfStats downloads={m.downloads} likes={m.likes} updated={m.last_modified} />
          </div>
          {#if file && fit}
            <div class="rounded-xl bg-card p-3">
              <h3 class="mb-1.5 text-xs font-semibold text-muted-foreground">On this computer</h3>
              <div class="flex items-center justify-between gap-2">
                <span class="text-sm font-medium">{quantOf(file.name) ?? "File"}</span>
                <Badge size="sm" variant={fitLabel[fit].variant}>{fitLabel[fit].text}</Badge>
              </div>
              <p class="mt-1.5 text-xs text-muted-foreground tabular-nums">
                Needs about {gb(memoryNeeded(file.size_bytes))} of memory, {gb(free)} free.
              </p>
              <p class="mt-1 text-xs text-muted-foreground">{fitNote[fit]}</p>
            </div>
          {/if}
          <p class="mt-auto text-xs text-muted-foreground">Check the license on Hugging Face before you use a model.</p>
        </aside>
        {/if}
      </div>

      {#if explore.downloadError}
        <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{explore.downloadError}</AlertDescription></Alert>
      {/if}
      {#if explore.download}
        <Progress
          value={explore.download.total > 0 ? (explore.download.downloaded / explore.download.total) * 100 : 0}
          indeterminate={explore.download.phase === "verifying" || explore.download.total === 0}
          helper={explore.download.phase === "verifying"
            ? "Verifying"
            : `${gb(explore.download.downloaded)} of ${gb(explore.download.total)} · ${(explore.download.bytesPerSecond / 1_048_576).toFixed(1)} MB/s`}
        />
      {/if}

      <DrawerFooter>
        {#if explore.download && file}
          <Button variant="secondary" onclick={() => explore.cancel(m.id, file)}>Cancel download</Button>
        {:else}
          <Button variant="ghost" onclick={() => explore.close()}>Close</Button>
          <Button disabled={!file || explore.loadingRepo} onclick={get}>{file ? `Download (${gb(total)})` : "Download"}</Button>
        {/if}
      </DrawerFooter>
    {/if}
  </DrawerContent>
</Drawer>
