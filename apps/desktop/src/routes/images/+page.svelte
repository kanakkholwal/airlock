<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { onMount } from "svelte";
  import {
    CheckCircleIcon,
    CpuIcon,
    DangerTriangleIcon,
    FolderOpenIcon,
    GalleryIcon,
    LockKeyholeIcon,
    MagnifierIcon,
    MagicWandIcon,
    PenNewSquareIcon,
    ShieldCheckIcon,
  } from "@solar-icons/svelte/linear";
  import type { ImageInfo } from "../../bindings";
  import { inTauri } from "#lib/env.js";
  import { formatBytes, formatDate } from "#lib/format.js";
  import { images } from "#lib/images.svelte.js";
  import { IDEAS, SHAPES, SORTS, filterImages, shapeCounts, type GallerySort, type ShapeId } from "#lib/image-presets.js";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import ConfirmDialog from "#lib/components/app/confirm-dialog.svelte";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";
  import ImageDetails from "#lib/components/app/image-details.svelte";
  import {
    ImageGallery,
    ImageGalleryEmpty,
    ImageGalleryGrid,
  } from "#lib/components/ui/image-gallery/index.js";
  import {
    ImageViewer,
    ImageViewerBody,
    ImageViewerClose,
    ImageViewerContent,
    ImageViewerCounter,
    ImageViewerDetailsToggle,
    ImageViewerFilmstrip,
    ImageViewerNext,
    ImageViewerPrevious,
    ImageViewerStage,
    ImageViewerToolbar,
    ImageViewerZoom,
  } from "#lib/components/ui/image-viewer/index.js";
  import { EmptyHeader } from "#lib/components/ui/empty/index.js";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import {
    ImageGeneration,
    ImageGenerationBadge,
    ImageGenerationCaption,
    ImageGenerationField,
    ImageGenerationFrame,
    ImageGenerationMedia,
    ImageGenerationPrompt,
    ImageGenerationRetry,
    ImageGenerationStatus,
    type ImageGenerationPhase,
  } from "#lib/components/agents/image-generation/index.js";
  import { Composer, ComposerInput, ComposerSubmit, ComposerToolbar } from "#lib/components/agents/composer/index.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#lib/components/ui/collapsible/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";
  import { Input } from "#lib/components/ui/input/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Stepper } from "#lib/components/ui/stepper/index.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";

  // Decision (product): two places, not one. Create is the studio: what is being made now,
  // large, with the box to describe the next one pinned at the bottom like chat. Gallery is
  // everything made before. Setup is a short guided path, not a settings page.
  const shapes = SHAPES;
  const ideas = IDEAS;
  const setupSteps = [
    { id: "engine", label: "Engine", description: "Runs the model on your GPU" },
    { id: "model", label: "Model", description: "Z-Image Turbo" },
    { id: "ready", label: "Ready" },
  ];

  let tab = $state("create");
  let prompt = $state("");
  let shape = $state<ShapeId>("square");
  let viewerIndex = $state(0);
  let viewerOpen = $state(false);
  let deleting = $state<ImageInfo | null>(null);
  let deleteOpen = $state(false);
  let lastRequest = $state<{ prompt: string; shape: string } | null>(null);

  onMount(() => void images.init());

  const size = $derived(shapes.find((s) => s.value === shape) ?? shapes[0]);
  const src = (i: ImageInfo) => convertFileSrc(i.path);
  const ratio = (w: number, h: number) => `${w} / ${h}`;

  // Where setup stands: engine first, then model, then done.
  const setupCurrent = $derived(!images.hasEngine ? 0 : images.downloaded.length === 0 ? 1 : 2);
  const setupBytes = $derived(
    (images.hasEngine ? 0 : (images.recommendedEngine?.size_bytes ?? 0)) + (images.recommendedModel?.missing_bytes ?? 0),
  );
  const activeDownload = $derived.by(() => {
    const [key, dl] = Object.entries(images.downloads)[0] ?? [];
    if (!key || !dl) return null;
    const engine = key.startsWith("engine:");
    return {
      ...dl,
      key,
      title: engine ? "Image engine" : (images.models.find((m) => m.id === key)?.name ?? "Image model"),
      detail: engine ? "Step 1 of 2" : "Step 2 of 2",
    };
  });

  const phase = $derived.by<ImageGenerationPhase>(() => {
    const run = images.run;
    if (!run) return images.last ? "complete" : images.error ? "error" : "generating";
    if (run.phase === "queued") return "queued";
    if (run.phase === "making" && run.total > 0 && run.step / run.total > 0.5) return "refining";
    return "generating";
  });
  const showStage = $derived(images.run !== null || images.last !== null || (images.error !== null && lastRequest !== null));
  const stageSize = $derived(
    images.run
      ? { w: images.run.width, h: images.run.height }
      : images.last
        ? { w: images.last.width, h: images.last.height }
        : { w: size.width, h: size.height },
  );
  const stagePrompt = $derived(images.run?.prompt ?? images.last?.prompt ?? lastRequest?.prompt ?? "");
  const stageLabels = $derived({
    queued: "Waiting for the chat reply to finish",
    generating:
      images.run?.phase === "making"
        ? `Making your image, step ${images.run.step} of ${images.run.total}`
        : (images.run?.log.at(-1) ?? "Getting ready"),
    refining: images.run ? `Adding detail, step ${images.run.step} of ${images.run.total}` : "Adding detail",
    complete: "Done",
    error: "Could not make this one",
  });

  // Steps come from what the engine reports: waiting or preparing, then the drawing steps, then saving.
  const progressSteps = $derived.by(() => {
    const run = images.run;
    const failed = images.failedPhase;
    const drawing =
      run?.phase === "making" && run.total > 0 ? `Step ${run.step} of ${run.total}` : "Draws the picture in small steps";
    const prepare =
      run?.phase === "queued"
        ? "Waiting for the chat reply to finish"
        : (run?.log.at(-1) ?? "Loads the model and reads your prompt");
    return [
      {
        id: "prepare",
        label: "Prepare",
        description: prepare,
        error: failed === "queued" || failed === "starting" ? (images.error ?? "Failed") : undefined,
      },
      {
        id: "draw",
        label: "Draw",
        description: drawing,
        error: failed === "making" ? (images.error ?? "Failed") : undefined,
      },
      { id: "save", label: "Save", description: "Writes the picture to your Gallery" },
    ];
  });
  const progressCurrent = $derived.by(() => {
    const run = images.run;
    if (run) return run.phase === "making" ? (run.total > 0 && run.step >= run.total ? 2 : 1) : 0;
    if (images.last) return 3;
    return images.failedPhase === "making" ? 1 : 0;
  });

  let now = $state(Date.now());
  $effect(() => {
    if (!images.run) return;
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });
  const elapsed = $derived(images.run ? Math.max(0, Math.round((now - images.run.startedAt) / 1000)) : 0);

  function make(text: string) {
    const value = text.trim();
    if (!value || images.busy) return;
    lastRequest = { prompt: value, shape };
    void images.generate(value, size.width, size.height);
  }

  function retry() {
    if (!lastRequest) return;
    const s = shapes.find((x) => x.value === lastRequest?.shape) ?? shapes[0];
    void images.generate(lastRequest.prompt, s.width, s.height);
  }

  // Gallery view: what shows is whatever the data and the filters say. The viewer walks the same
  // list, so previous and next never jump to a picture that is filtered out.
  let galleryQuery = $state("");
  let galleryShape = $state<ShapeId | "all">("all");
  let gallerySort = $state<GallerySort>("newest");
  const visible = $derived(filterImages(images.gallery, { query: galleryQuery, shape: galleryShape, sort: gallerySort }));
  const shapeOptions = $derived(shapeCounts(images.gallery));
  const filtering = $derived(galleryQuery.trim() !== "" || galleryShape !== "all");
  function clearFilters() {
    galleryQuery = "";
    galleryShape = "all";
  }

  // The registry viewer and gallery take plain pictures; the originals stay in `visible`.
  const pictures = $derived(
    visible.map((i) => ({
      src: src(i),
      alt: i.prompt,
      width: i.width,
      height: i.height,
      caption: `${i.width} × ${i.height} · ${formatDate(i.created_ms)}`,
    })),
  );

  function open(image: ImageInfo) {
    // A picture just made may be filtered out of the gallery, so the viewer opens on the full list then.
    if (!visible.some((i) => i.id === image.id)) clearFilters();
    viewerIndex = Math.max(0, visible.findIndex((i) => i.id === image.id));
    viewerOpen = true;
  }

  function reuse(image: ImageInfo) {
    viewerOpen = false;
    prompt = image.prompt;
    const match = shapes.find((s) => s.width === image.width && s.height === image.height);
    if (match) shape = match.value;
    tab = "create";
  }

  function askDelete(image: ImageInfo) {
    deleting = image;
    deleteOpen = true;
  }
</script>

<div class="flex h-full flex-col">
  {#if !inTauri()}
    <div class="m-auto max-w-md px-6">
      <StateView
        icon={GalleryIcon}
        title="Open the Airlock app to make images"
      />
    </div>
  {:else if !images.loaded}
    <div class="mx-auto w-full max-w-3xl px-6 py-14"><CardSkeleton count={2} /></div>
  {:else if !images.supported}
    <div class="m-auto max-w-md px-6">
      <StateView
        icon={DangerTriangleIcon}
        tone="warning"
        title="Images are not available on this system yet" description="Chat and dictation still work."
      />
    </div>
  {:else if !images.ready}
    <PageScroll>
      <div class="mx-auto flex w-full max-w-xl flex-col gap-6 px-6 py-14">
        <header class="text-center">
          <span class="mx-auto mb-4 flex size-12 items-center justify-center rounded-2xl bg-primary/10 text-primary">
            <GalleryIcon size={24} />
          </span>
          <h1 class="text-2xl text-balance">Make images on this computer</h1>
          <p class="mt-2 text-sm text-muted-foreground text-pretty">
            Two downloads, then pictures from text.
          </p>
        </header>

        <Stepper steps={setupSteps} current={setupCurrent} />

        {#if activeDownload}
          <DownloadProgress
            title={activeDownload.title}
            detail={activeDownload.detail}
            downloaded={activeDownload.downloaded}
            total={activeDownload.total}
            bytesPerSecond={activeDownload.bytesPerSecond}
            phase={activeDownload.phase}
            oncancel={() => images.cancelDownload(activeDownload.key)}
          />
        {:else}
          <div class="flex flex-col items-center gap-3">
            <Button size="lg" class="gap-2" onclick={() => images.runSetup()} disabled={images.settingUp}>
              <MagicWandIcon size={18} />Set up for {formatBytes(setupBytes)}
            </Button>
            <ul class="flex flex-wrap justify-center gap-x-5 gap-y-1 text-sm text-muted-foreground">
              <li class="flex items-center gap-1.5"><LockKeyholeIcon size={14} />Private</li>
              <li class="flex items-center gap-1.5"><CpuIcon size={14} />Uses your GPU</li>
              <li class="flex items-center gap-1.5"><ShieldCheckIcon size={14} />Checked on download</li>
            </ul>
          </div>
        {/if}

        {#if images.error}
          <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription class="whitespace-pre-line">{images.error}</AlertDescription></Alert>
        {/if}
      </div>
    </PageScroll>
  {:else}
    <div class="flex shrink-0 justify-center pt-3">
      <Tabs bind:value={tab} variant="segment">
        <TabsList>
          <TabsTrigger value="create">Create</TabsTrigger>
          <TabsTrigger value="gallery">
            Gallery{#if images.gallery.length > 0}<span class="ms-1.5 text-muted-foreground tabular-nums">{images.gallery.length}</span>{/if}
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    {#if tab === "create"}
      <div class="min-h-0 flex-1">
        <PageScroll>
          <div class="mx-auto flex w-full max-w-3xl flex-col gap-5 px-6 py-6">
            {#if showStage}
              <!-- Width is capped so the picture is never taller than 62% of the window. -->
              <div
                class="mx-auto w-full motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300"
                style:max-width="min(100%, calc(62dvh * {stageSize.w / stageSize.h}))"
              >
                <ImageGeneration status={phase} variant="fluid">
                  <ImageGenerationFrame label={stagePrompt || "Image"} aspectRatio={ratio(stageSize.w, stageSize.h)}>
                    <ImageGenerationMedia>
                      {#if images.last}
                        <button type="button" class="size-full cursor-zoom-in" aria-label="Open larger" onclick={() => images.last && open(images.last)}>
                          <img src={src(images.last)} alt={images.last.prompt} />
                        </button>
                      {/if}
                    </ImageGenerationMedia>
                    <ImageGenerationField />
                    <ImageGenerationBadge>{stageSize.w} × {stageSize.h}</ImageGenerationBadge>
                  </ImageGenerationFrame>
                  <ImageGenerationCaption>
                    <ImageGenerationStatus labels={stageLabels} />
                    <ImageGenerationPrompt>{stagePrompt}</ImageGenerationPrompt>
                  </ImageGenerationCaption>
                  <ImageGenerationRetry onclick={retry} />
                </ImageGeneration>
              </div>

              <!-- Decision (product): progress is one slim row under the picture, not a panel. The
                   caption above already says what is happening; this shows where it is in the run. -->
              <div class="mx-auto flex w-full max-w-md items-center justify-between gap-4 text-sm">
                <ol class="flex items-center gap-3" aria-label="Progress">
                  {#each progressSteps as step, i (step.id)}
                    {@const done = i < progressCurrent}
                    {@const now = i === progressCurrent && images.run !== null}
                    <li class="flex items-center gap-1.5 {step.error ? 'text-destructive-strong' : done || now ? 'text-foreground' : 'text-muted-foreground'}">
                      {#if step.error}
                        <DangerTriangleIcon size={14} />
                      {:else if done}
                        <CheckCircleIcon size={14} class="text-success-strong" />
                      {:else if now}
                        <Spinner size="sm" label={step.label} />
                      {:else}
                        <span class="size-1.5 rounded-full bg-current opacity-40" aria-hidden="true"></span>
                      {/if}
                      <span class={now ? "font-medium" : ""}>{step.label}</span>
                    </li>
                    {#if i < progressSteps.length - 1}<span class="h-px w-4 bg-border" aria-hidden="true"></span>{/if}
                  {/each}
                </ol>
                {#if images.run}
                  <span class="text-muted-foreground tabular-nums">{elapsed} s</span>
                {:else if images.last}
                  <Badge size="sm" variant="success">Done</Badge>
                {:else}
                  <Badge size="sm" variant="destructive">Stopped</Badge>
                {/if}
              </div>

              {#if images.error && !images.run}
                <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription class="whitespace-pre-line break-words">{images.error}</AlertDescription></Alert>
              {/if}

              {#if images.run && images.run.log.length > 0}
                <Collapsible class="mx-auto w-full max-w-md">
                  <CollapsibleTrigger class="text-xs text-muted-foreground transition-colors hover:text-foreground">
                    What the model is doing
                  </CollapsibleTrigger>
                  <CollapsibleContent>
                    <ol class="mt-2 flex max-h-40 flex-col gap-0.5 overflow-y-auto rounded-xl bg-muted p-3 font-mono text-xs text-muted-foreground">
                      {#each images.run.log.slice(-12) as line, i (i + line)}
                        <li class="truncate">{line}</li>
                      {/each}
                    </ol>
                  </CollapsibleContent>
                </Collapsible>
              {/if}

              {#if images.last && !images.run}
                <div class="flex flex-wrap justify-center gap-2">
                  <Button size="sm" variant="secondary" class="gap-1.5" onclick={() => images.last && revealItemInDir(images.last.path)}>
                    <FolderOpenIcon size={16} />Show in folder
                  </Button>
                  <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => images.last && open(images.last)}>
                    <GalleryIcon size={16} />Details
                  </Button>
                  <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => images.last && (prompt = images.last.prompt)}>
                    <PenNewSquareIcon size={16} />Edit prompt
                  </Button>
                </div>
              {/if}
            {:else}
              <!-- Decision (product): an empty studio offers ways in. The ideas fill the box. -->
              <div class="flex flex-col items-center gap-6 pt-10 pb-4 text-center motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
                <span class="flex size-12 items-center justify-center rounded-2xl bg-primary/10 text-primary">
                  <MagicWandIcon size={24} />
                </span>
                <div>
                  <h2 class="text-xl">What should we make?</h2>
                  <p class="mt-1 text-sm text-muted-foreground">Describe it below, or start from an idea.</p>
                </div>
                <ul class="flex w-full flex-col gap-2 sm:flex-row" aria-label="Ideas">
                  {#each ideas as idea, i (idea)}
                    <li
                      class="flex-1 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2 motion-safe:fill-mode-backwards motion-safe:duration-300"
                      style:animation-delay="{i * 50}ms"
                    >
                      <button
                        type="button"
                        class="flex h-full w-full items-start gap-2 rounded-xl bg-card p-3.5 text-start text-sm transition-[background-color,scale] duration-150 hover:bg-muted active:scale-[0.98] focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none"
                        onclick={() => (prompt = idea)}
                      >
                        <MagicWandIcon size={16} class="mt-0.5 shrink-0 text-muted-foreground" />
                        <span class="text-pretty">{idea}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
                {#if images.error && !images.run}
                  <Alert variant="destructive" class="text-start"><DangerTriangleIcon size={18} /><AlertDescription class="whitespace-pre-line">{images.error}</AlertDescription></Alert>
                {/if}
              </div>
            {/if}
          </div>
        </PageScroll>
      </div>

      <div class="mx-auto w-full max-w-3xl shrink-0 px-6 pb-5">
        <section class="rounded-2xl bg-card p-2">
          <Composer bind:value={prompt} variant="filled" loading={images.busy} onStop={() => images.cancel()} onSubmit={make}>
            <ComposerInput placeholder="Describe the picture you want" minRows={2} />
            <ComposerToolbar>
              <ToggleGroup type="single" variant="segmented" size="sm" bind:value={shape} label="Shape" disabled={images.busy}>
                {#each shapes as s (s.value)}
                  <ToggleGroupItem value={s.value} class="gap-1.5">
                    <span class="rounded-[3px] border-[1.5px] border-current {s.box}"></span>{s.label}
                  </ToggleGroupItem>
                {/each}
              </ToggleGroup>
              <div class="ms-auto flex items-center gap-2">
                {#if images.model}<Badge size="sm" variant="secondary">{images.model.name}</Badge>{/if}
                <ComposerSubmit />
              </div>
            </ComposerToolbar>
          </Composer>
        </section>
      </div>
    {:else}
      <div class="min-h-0 flex-1">
        <PageScroll>
          <div class="mx-auto flex w-full max-w-5xl flex-col gap-5 px-6 py-6">
            {#if images.gallery.length > 0}
              <div class="flex flex-wrap items-center gap-2">
                <div class="relative min-w-48 flex-1">
                  <MagnifierIcon size={16} class="pointer-events-none absolute start-3 top-1/2 -translate-y-1/2 text-muted-foreground" />
                  <Input class="ps-9" placeholder="Search your prompts" aria-label="Search pictures by prompt" bind:value={galleryQuery} />
                </div>
                {#if shapeOptions.length > 1}
                  <ToggleGroup type="single" variant="segmented" size="sm" bind:value={galleryShape} label="Shape">
                    <ToggleGroupItem value="all">All</ToggleGroupItem>
                    {#each shapeOptions as o (o.value)}
                      <ToggleGroupItem value={o.value}>{o.label}<span class="ms-1.5 text-muted-foreground tabular-nums">{o.count}</span></ToggleGroupItem>
                    {/each}
                  </ToggleGroup>
                {/if}
                <Select items={SORTS} bind:value={gallerySort}>
                  <SelectTrigger class="w-40"><span class="truncate">{SORTS.find((o) => o.value === gallerySort)?.label}</span></SelectTrigger>
                  <SelectContent align="end">
                    {#each SORTS as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
                  </SelectContent>
                </Select>
              </div>
              <p class="px-1 text-xs text-muted-foreground tabular-nums" aria-live="polite">
                {visible.length} of {images.gallery.length} {images.gallery.length === 1 ? "picture" : "pictures"}
              </p>
            {/if}

            {#if filtering && visible.length === 0}
              <StateView icon={MagnifierIcon} title="No pictures match" description="Try another word or shape.">
                {#snippet actions()}<Button variant="secondary" onclick={clearFilters}>Clear filters</Button>{/snippet}
              </StateView>
            {:else}
              <ImageGallery images={pictures} onOpen={(index) => open(visible[index])}>
                <ImageGalleryGrid />
                <ImageGalleryEmpty>
                  <EmptyHeader>
                    <p class="text-base font-medium">Nothing here yet</p>
                    <p class="text-sm text-muted-foreground">Pictures you make show up here, saved on this computer.</p>
                  </EmptyHeader>
                  <Button class="gap-2" onclick={() => (tab = "create")}><MagicWandIcon size={16} />Make your first image</Button>
                </ImageGalleryEmpty>
              </ImageGallery>
            {/if}
          </div>
        </PageScroll>
      </div>
    {/if}
  {/if}
</div>

<ImageViewer variant="themed" images={pictures} bind:open={viewerOpen} bind:index={viewerIndex} loop>
  <ImageViewerContent>
    <ImageViewerToolbar>
      <ImageViewerPrevious />
      <ImageViewerCounter />
      <ImageViewerNext />
      <ImageViewerZoom class="mx-auto" />
      <ImageViewerDetailsToggle />
      <ImageViewerClose />
    </ImageViewerToolbar>
    <ImageViewerBody>
      <ImageViewerStage />
      <ImageDetails images={visible} onreuse={reuse} ondelete={askDelete} />
    </ImageViewerBody>
    <ImageViewerFilmstrip />
  </ImageViewerContent>
</ImageViewer>

<ConfirmDialog
  bind:open={deleteOpen}
  title="Delete this image?"
  description="The picture is removed from this computer and cannot be brought back."
  onconfirm={() => {
    if (deleting) void images.remove(deleting.id);
  }}
/>
