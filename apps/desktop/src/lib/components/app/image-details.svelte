<script lang="ts">
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { FolderOpenIcon, MagicWandIcon, TrashBin2Icon } from "@solar-icons/svelte/linear";
  import type { ImageInfo } from "../../../bindings";
  import { formatDate } from "#lib/format.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import { ImageViewerDetails, ImageViewerTitle, useImageViewer } from "#lib/components/ui/image-viewer/index.js";

  // What goes in the Image Viewer's details panel: the prompt, how the picture was made, and what
  // you can do with it. A picture on this computer is a file, so the action is "show in folder".
  let {
    images,
    onreuse,
    ondelete,
  }: {
    images: ImageInfo[];
    onreuse: (image: ImageInfo) => void;
    ondelete: (image: ImageInfo) => void;
  } = $props();

  const viewer = useImageViewer();
  const image = $derived(images[viewer.index]);
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
</script>

<ImageViewerDetails>
  {#if image}
    <div class="flex h-full flex-col gap-4">
      <div class="flex flex-col gap-1.5">
        <ImageViewerTitle class="text-sm text-muted-foreground">Prompt</ImageViewerTitle>
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
          onclick={() => onreuse(image)}
        >
          <MagicWandIcon size={16} />Use this prompt again
        </Button>
        <div class="flex gap-2">
          <CopyButton text={image.prompt} label="Copy prompt" class="flex-1" />
          <Button variant="ghost" size="icon" aria-label="Delete image" onclick={() => ondelete(image)}>
            <TrashBin2Icon size={16} />
          </Button>
        </div>
      </div>
    </div>
  {/if}
</ImageViewerDetails>
