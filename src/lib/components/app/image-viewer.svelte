<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import {
    AltArrowLeftIcon,
    AltArrowRightIcon,
    FolderOpenIcon,
    FullScreenIcon,
    InfoCircleIcon,
    MagicWandIcon,
    RoundedMagnifierZoomInIcon,
    RoundedMagnifierZoomOutIcon,
    TrashBin2Icon,
  } from "@solar-icons/svelte/linear";
  import type { ImageInfo } from "../../../bindings";
  import { formatDate } from "#lib/format.js";
  import { actualPixelsScale, clampView, containSize, FIT, zoomAt, type Size, type View } from "#lib/zoom.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import { Sheet, SheetClose, SheetContent, SheetTitle } from "#lib/components/ui/sheet/index.js";
  import { Tooltip, TooltipContent, TooltipTrigger } from "#lib/components/ui/tooltip/index.js";

  // Decision (product): a picture is looked at, not skimmed, so it gets the whole right side of
  // the window. The stage zooms (wheel, buttons, keys, double click) and pans by dragging; the
  // details sit beside it and can be hidden to give the picture every pixel. A local picture is
  // already a file, so the action is "show in folder", never "download".
  let {
    images,
    id = $bindable(null),
    open = $bindable(false),
    onreuse,
    ondelete,
  }: {
    images: ImageInfo[];
    id?: string | null;
    open?: boolean;
    onreuse: (image: ImageInfo) => void;
    ondelete: (image: ImageInfo) => void;
  } = $props();

  const index = $derived(images.findIndex((i) => i.id === id));
  const image = $derived(index >= 0 ? images[index] : null);

  let stage = $state<Size>({ w: 0, h: 0 });
  let natural = $state<Size>({ w: 0, h: 0 });
  let view = $state<View>(FIT);
  let showInfo = $state(true);
  let dragging = $state(false);
  let drag: { x: number; y: number; vx: number; vy: number } | null = null;

  const base = $derived(containSize(natural, stage));
  const percent = $derived(base.w > 0 ? Math.round(((base.w * view.scale) / natural.w) * 100) : 100);
  const actual = $derived(actualPixelsScale(natural, base));
  const facts = $derived(
    image
      ? [
          { label: "Size", value: `${image.width} × ${image.height}` },
          { label: "Steps", value: String(image.steps) },
          { label: "Seed", value: String(image.seed) },
          { label: "Model", value: image.model },
          { label: "Made", value: formatDate(image.created_ms) },
        ]
      : [],
  );

  // A new picture starts fitted, and its size is read again once it has loaded.
  $effect(() => {
    void id;
    view = FIT;
    natural = { w: 0, h: 0 };
  });

  const apply = (v: View) => (view = clampView(v, base, stage));
  const zoom = (factor: number, point = { x: 0, y: 0 }) => apply(zoomAt(view, factor, point, base, stage));

  function fromCenter(e: { clientX: number; clientY: number }, el: HTMLElement) {
    const r = el.getBoundingClientRect();
    return { x: e.clientX - r.left - r.width / 2, y: e.clientY - r.top - r.height / 2 };
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    zoom(Math.exp(-e.deltaY * 0.0015), fromCenter(e, e.currentTarget as HTMLElement));
  }

  function onDown(e: PointerEvent) {
    if (view.scale <= 1) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { x: e.clientX, y: e.clientY, vx: view.x, vy: view.y };
    dragging = true;
  }

  function onMove(e: PointerEvent) {
    if (!drag) return;
    apply({ scale: view.scale, x: drag.vx + e.clientX - drag.x, y: drag.vy + e.clientY - drag.y });
  }

  function onUp() {
    drag = null;
    dragging = false;
  }

  function onDouble(e: MouseEvent) {
    if (view.scale > 1) view = FIT;
    else zoom(2.5, fromCenter(e, e.currentTarget as HTMLElement));
  }

  function step(delta: number) {
    const next = images[index + delta];
    if (next) id = next.id;
  }

  function onKey(e: KeyboardEvent) {
    if (!open || (e.target as HTMLElement | null)?.closest("input, textarea")) return;
    if (e.key === "+" || e.key === "=") zoom(1.4);
    else if (e.key === "-") zoom(1 / 1.4);
    else if (e.key === "0") view = FIT;
    else if (e.key === "ArrowLeft" && view.scale === 1) step(-1);
    else if (e.key === "ArrowRight" && view.scale === 1) step(1);
    else return;
    e.preventDefault();
  }

  function remove(img: ImageInfo) {
    // Show the next picture, or close when this was the last one.
    const fallback = images[index + 1] ?? images[index - 1];
    if (fallback) id = fallback.id;
    else open = false;
    ondelete(img);
  }
</script>

<svelte:window onkeydown={onKey} />

{#snippet tool(label: string, onclick: () => void, disabled = false, icon?: import("svelte").Snippet)}
  <Tooltip>
    <TooltipTrigger>
      {#snippet child({ props })}
        <Button {...props} variant="ghost" size="icon" aria-label={label} {disabled} {onclick}>
          {@render icon?.()}
        </Button>
      {/snippet}
    </TooltipTrigger>
    <TooltipContent>{label}</TooltipContent>
  </Tooltip>
{/snippet}

{#snippet zoomOutIcon()}<RoundedMagnifierZoomOutIcon size={18} />{/snippet}
{#snippet zoomInIcon()}<RoundedMagnifierZoomInIcon size={18} />{/snippet}
{#snippet fitIcon()}<FullScreenIcon size={18} />{/snippet}
{#snippet infoIcon()}<InfoCircleIcon size={18} />{/snippet}
{#snippet prevIcon()}<AltArrowLeftIcon size={18} />{/snippet}
{#snippet nextIcon()}<AltArrowRightIcon size={18} />{/snippet}

<Sheet bind:open>
  <SheetContent side="right" class="w-[min(80rem,100vw)] p-0">
    {#if image}
      <SheetTitle class="sr-only">Picture</SheetTitle>
      <div class="flex h-full min-h-0">
        <div class="flex min-w-0 flex-1 flex-col">
          <div class="flex h-12 shrink-0 items-center gap-1 px-3">
            {@render tool("Previous picture", () => step(-1), index <= 0, prevIcon)}
            <span class="min-w-14 text-center text-sm text-muted-foreground tabular-nums">{index + 1} of {images.length}</span>
            {@render tool("Next picture", () => step(1), index >= images.length - 1, nextIcon)}

            <div class="mx-auto flex items-center gap-1">
              {@render tool("Zoom out", () => zoom(1 / 1.4), view.scale <= 1, zoomOutIcon)}
              <button
                type="button"
                class="min-w-14 rounded-lg px-2 py-1 text-center text-sm tabular-nums text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                aria-label="Fit to window"
                onclick={() => (view = FIT)}
              >
                {percent}%
              </button>
              {@render tool("Zoom in", () => zoom(1.4), view.scale >= 8, zoomInIcon)}
              {@render tool("Fit to window", () => (view = FIT), view.scale === 1, fitIcon)}
              {#if actual}
                <Button variant="ghost" size="sm" onclick={() => apply({ scale: actual, x: 0, y: 0 })}>100%</Button>
              {/if}
            </div>

            {@render tool(showInfo ? "Hide details" : "Show details", () => (showInfo = !showInfo), false, infoIcon)}
            <SheetClose />
          </div>

          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="relative min-h-0 flex-1 touch-none overflow-hidden bg-muted/60 {view.scale > 1 ? (dragging ? 'cursor-grabbing' : 'cursor-grab') : 'cursor-zoom-in'}"
            bind:clientWidth={stage.w}
            bind:clientHeight={stage.h}
            onwheel={onWheel}
            onpointerdown={onDown}
            onpointermove={onMove}
            onpointerup={onUp}
            onpointercancel={onUp}
            ondblclick={onDouble}
          >
            {#key image.id}
              <img
                src={convertFileSrc(image.path)}
                alt={image.prompt}
                draggable="false"
                onload={(e) => {
                  const el = e.currentTarget as HTMLImageElement;
                  natural = { w: el.naturalWidth, h: el.naturalHeight };
                }}
                style:width="{base.w}px"
                style:height="{base.h}px"
                style:transform="translate({view.x}px, {view.y}px) scale({view.scale})"
                class="absolute top-1/2 left-1/2 max-w-none -translate-x-1/2 -translate-y-1/2 select-none rounded-sm motion-safe:animate-in motion-safe:fade-in motion-safe:duration-200 {dragging ? '' : 'transition-transform duration-150 ease-out motion-reduce:transition-none'}"
              />
            {/key}
          </div>
          <p class="shrink-0 px-4 py-2 text-center text-xs text-muted-foreground">
            Scroll to zoom, drag to move, double click to zoom in or out. Arrow keys switch pictures.
          </p>
        </div>

        <!-- The wrapper animates its width; the panel inside keeps its own, so text never reflows. -->
        <div
          class="hidden shrink-0 overflow-hidden transition-[width,opacity] duration-300 ease-out motion-reduce:transition-none md:block {showInfo ? 'w-72 opacity-100' : 'w-0 opacity-0'}"
          inert={!showInfo}
        >
          <aside class="flex h-full w-72 shrink-0 flex-col gap-4 overflow-y-auto p-5">
            <div class="flex flex-col gap-1.5">
              <h2 class="text-sm text-muted-foreground">Prompt</h2>
              <p class="text-pretty">{image.prompt}</p>
            </div>
            <dl class="flex flex-col gap-2 text-sm">
              {#each facts as f (f.label)}
                <div class="flex items-baseline justify-between gap-3">
                  <dt class="text-muted-foreground">{f.label}</dt>
                  <dd class="min-w-0 truncate text-right tabular-nums" title={f.value}>{f.value}</dd>
                </div>
              {/each}
            </dl>
            <div class="mt-auto flex flex-col gap-2">
              <Button class="justify-start gap-2" onclick={() => revealItemInDir(image.path)}>
                <FolderOpenIcon size={16} />Show in folder
              </Button>
              <Button
                variant="secondary"
                class="justify-start gap-2"
                onclick={() => {
                  open = false;
                  onreuse(image);
                }}
              >
                <MagicWandIcon size={16} />Use this prompt again
              </Button>
              <div class="flex gap-2">
                <CopyButton text={image.prompt} label="Copy prompt" class="flex-1" />
                <Button variant="ghost" size="icon" aria-label="Delete image" onclick={() => remove(image)}>
                  <TrashBin2Icon size={16} />
                </Button>
              </div>
            </div>
          </aside>
        </div>
      </div>
    {/if}
  </SheetContent>
</Sheet>
