<script lang="ts">
  import { DownloadMinimalisticIcon, ShieldCheckIcon } from "@solar-icons/svelte/linear";
  import { formatBytes, formatEta, formatSpeed } from "#lib/format.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";

  // Decision (product): every download, whatever it is, shows the same four facts: what, how
  // far, how fast and how long. The icon changes when the file is being checked, so a pause at
  // 100% reads as work, not a hang.
  let {
    title,
    detail,
    downloaded,
    total,
    bytesPerSecond = 0,
    phase = "downloading",
    oncancel,
    compact = false,
  }: {
    title: string;
    detail?: string;
    downloaded: number;
    total: number;
    bytesPerSecond?: number;
    phase?: "downloading" | "verifying";
    oncancel?: () => void;
    /** Inside a card that already names the file: just the bar and the numbers. */
    compact?: boolean;
  } = $props();

  const percent = $derived(total > 0 ? Math.min(100, (downloaded / total) * 100) : 0);
  const eta = $derived(formatEta(Math.max(0, total - downloaded), bytesPerSecond));
</script>

<div class={compact ? "flex flex-col gap-2" : "flex flex-col gap-3 rounded-2xl bg-card p-4"} role="status" aria-live="polite">
  {#if !compact}
  <div class="flex items-center gap-3">
    <span
      class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary"
      aria-hidden="true"
    >
      {#if phase === "verifying"}
        <ShieldCheckIcon size={18} class="animate-pulse" />
      {:else}
        <DownloadMinimalisticIcon size={18} class="motion-safe:animate-bounce" />
      {/if}
    </span>
    <div class="min-w-0 flex-1">
      <p class="truncate text-sm font-medium">{title}</p>
      {#if detail}<p class="truncate text-sm text-muted-foreground">{detail}</p>{/if}
    </div>
    <span class="text-sm tabular-nums text-muted-foreground">
      {phase === "verifying" ? "Checking" : `${Math.round(percent)}%`}
    </span>
  </div>
  {/if}

  <Progress value={percent} indeterminate={phase === "verifying" || total === 0} label={title} />

  <div class="flex items-center justify-between gap-3 text-sm text-muted-foreground">
    <p class="min-w-0 truncate tabular-nums">
      {#if phase === "verifying"}
        Making sure the file is intact
      {:else if total > 0}
        {formatBytes(downloaded)} of {formatBytes(total)}
        {#if bytesPerSecond > 0} · {formatSpeed(bytesPerSecond)}{/if}
        {#if eta} · {eta}{/if}
      {:else}
        Starting
      {/if}
    </p>
    {#if oncancel && phase === "downloading"}
      <Button size="sm" variant="ghost" onclick={oncancel}>Cancel</Button>
    {/if}
  </div>
</div>
