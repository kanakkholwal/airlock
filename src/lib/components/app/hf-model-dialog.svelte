<script lang="ts">
  import { toast } from "svelte-sonner";
  import { explore } from "#lib/explore.svelte.js";
  import { fitFor, quantOf, suggestFile, type Fit } from "#lib/hf-compat.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
  } from "#lib/components/ui/dialog/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";

  // Decision (product): the dialog preselects the file that fits this machine best, so the
  // common case is one click. Every file shows whether it fits, because the biggest quality
  // win is not worth a model that cannot load.
  const repo = $derived(explore.selected);
  const free = $derived(explore.memory?.free_bytes ?? 0);
  const models = $derived((repo?.files ?? []).filter((f) => !f.projector));
  const projectors = $derived((repo?.files ?? []).filter((f) => f.projector));

  let chosen = $state<string | null>(null);
  // Preselect once the files arrive.
  $effect(() => {
    if (repo && !chosen && models.length > 0) chosen = suggestFile(models, free)?.name ?? null;
    if (!repo) chosen = null;
  });

  const file = $derived(models.find((f) => f.name === chosen) ?? null);
  // A vision model needs its projector; the smallest one is enough and is fetched with it.
  const projector = $derived(repo?.model.vision && projectors.length > 0 ? projectors[0] : null);
  const total = $derived((file?.size_bytes ?? 0) + (projector?.size_bytes ?? 0));
  const visionBlocked = $derived(!!repo?.model.vision && !projector && !explore.loadingRepo);

  const gb = (b: number) => (b >= 1_073_741_824 ? `${(b / 1_073_741_824).toFixed(1)} GB` : `${Math.round(b / 1_048_576)} MB`);
  const fitLabel: Record<Fit, { text: string; variant: "success" | "warning" | "destructive" }> = {
    fits: { text: "Fits", variant: "success" },
    tight: { text: "Tight", variant: "warning" },
    "too-big": { text: "Too big", variant: "destructive" },
  };

  async function get() {
    if (!repo || !file) return;
    const ok = await explore.fetch(repo.model.id, file, projector);
    if (ok) {
      toast(`${repo.model.id.split("/")[1]} added`, { description: "Find it under Chat." });
      explore.close();
    }
  }
</script>

<!-- Open follows the store; closing is ignored while a download is running. -->
<Dialog
  bind:open={
    () => repo !== null,
    (v) => {
      if (!v && !explore.download) explore.close();
    }
  }
>
  <DialogContent class="max-w-lg">
    <DialogHeader>
      <DialogTitle>{repo?.model.id.split("/")[1] ?? "Model"}</DialogTitle>
      <DialogDescription>
        {repo?.model.id.split("/")[0]}{repo?.model.license ? ` · ${repo.model.license}` : ""}. Pick a file. Smaller
        files are faster and use less memory; larger ones are more accurate.
      </DialogDescription>
    </DialogHeader>

    {#if explore.loadingRepo}
      <div class="flex flex-col gap-2" role="status" aria-label="Loading files">
        {#each [0, 1, 2] as i (i)}<Skeleton class="h-12 rounded-xl" />{/each}
      </div>
    {:else if explore.repoError}
      <Alert variant="destructive"><AlertDescription>{explore.repoError}</AlertDescription></Alert>
    {:else if models.length === 0}
      <Alert variant="warning">
        <AlertDescription>This repository has no single-file GGUF that Airlock can download yet.</AlertDescription>
      </Alert>
    {:else}
      <div class="flex max-h-72 flex-col gap-1.5 overflow-y-auto" role="radiogroup" aria-label="Model file">
        {#each models as f (f.name)}
          {@const fit = fitLabel[fitFor(f.size_bytes, free)]}
          <button
            type="button"
            role="radio"
            aria-checked={chosen === f.name}
            disabled={explore.download !== null}
            onclick={() => (chosen = f.name)}
            class="flex items-center justify-between gap-3 rounded-xl px-3 py-2.5 text-start outline-none transition-colors duration-100 hover:bg-foreground/[0.06] focus-visible:ring-2 focus-visible:ring-ring aria-checked:bg-primary/10"
          >
            <span class="min-w-0">
              <span class="block truncate text-sm font-[450]">{quantOf(f.name) ?? f.name}</span>
              <span class="block truncate text-xs text-muted-foreground">{f.name}</span>
            </span>
            <span class="flex shrink-0 items-center gap-2">
              <span class="text-sm tabular-nums text-muted-foreground">{gb(f.size_bytes)}</span>
              <Badge size="sm" variant={fit.variant}>{fit.text}</Badge>
            </span>
          </button>
        {/each}
      </div>
      {#if projector}
        <p class="text-sm text-muted-foreground">Includes the vision projector ({gb(projector.size_bytes)}) so it can read images.</p>
      {/if}
      {#if visionBlocked}
        <Alert variant="warning">
          <AlertDescription>This is a vision model but the repository has no projector file, so it would only read text.</AlertDescription>
        </Alert>
      {/if}
    {/if}

    {#if explore.downloadError}
      <Alert variant="destructive"><AlertDescription>{explore.downloadError}</AlertDescription></Alert>
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

    <DialogFooter>
      {#if explore.download && repo && file}
        <Button variant="secondary" onclick={() => explore.cancel(repo.model.id, file)}>Cancel download</Button>
      {:else}
        <Button variant="ghost" onclick={() => explore.close()}>Close</Button>
        <Button disabled={!file || explore.loadingRepo} onclick={get}>
          {file ? `Download (${gb(total)})` : "Download"}
        </Button>
      {/if}
    </DialogFooter>
  </DialogContent>
</Dialog>
