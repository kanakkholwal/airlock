<script lang="ts">
  import {
    DownloadCard,
    DownloadCardCancel,
    DownloadCardDetail,
    DownloadCardFooter,
    DownloadCardHeader,
    DownloadCardIcon,
    DownloadCardPercent,
    DownloadCardProgress,
    DownloadCardStats,
    DownloadCardText,
    DownloadCardTitle,
  } from "#lib/components/ui/download-card/index.js";

  // One place that turns Airlock's download state into the Baby UI Download Card, so every
  // download in the app, from an engine to a voice pack, reads the same way.
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
</script>

<DownloadCard
  {downloaded}
  {total}
  {bytesPerSecond}
  phase={phase === "verifying" ? "checking" : "downloading"}
  variant={compact ? "compact" : "default"}
>
  {#if !compact}
    <DownloadCardHeader>
      <DownloadCardIcon />
      <DownloadCardText>
        <DownloadCardTitle>{title}</DownloadCardTitle>
        {#if detail}<DownloadCardDetail>{detail}</DownloadCardDetail>{/if}
      </DownloadCardText>
      <DownloadCardPercent />
    </DownloadCardHeader>
  {/if}
  <DownloadCardProgress label={title} />
  <DownloadCardFooter>
    <DownloadCardStats />
    {#if oncancel}<DownloadCardCancel onclick={oncancel} />{/if}
  </DownloadCardFooter>
</DownloadCard>
