<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open as pickFiles } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { DangerTriangleIcon, UploadMinimalisticIcon } from "@solar-icons/svelte/linear";
  import { inTauri } from "#lib/env.js";
  import { speech } from "#lib/speech.svelte.js";
  import {
    TranscriptRow,
    TranscriptRowActions,
    TranscriptRowCancel,
    TranscriptRowCopy,
    TranscriptRowError,
    TranscriptRowHeader,
    TranscriptRowProgress,
    TranscriptRowRetry,
    TranscriptRowStatusBadge,
    TranscriptRowTitle,
  } from "#lib/components/agents/transcript-row/index.js";
  import StateView from "#lib/components/app/state-view.svelte";
  import TranscriptView from "#lib/components/app/transcript-view.svelte";
  import { Switch } from "#lib/components/ui/switch/index.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Button } from "#lib/components/ui/button/index.js";

  // Decision (product): turn a recording, meeting or voice memo into text without leaving the
  // computer. Files join History, so the text is kept and findable; this tab shows what is in
  // flight. Speaker labels and summaries are a later stage, so they are not promised here.
  const EXTENSIONS = ["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus", "webm", "mp4", "mov", "mkv", "aiff"];
  let dragging = $state(false);

  onMount(() => {
    if (!inTauri()) return;
    let stop: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") dragging = true;
        else if (p.type === "leave") dragging = false;
        else if (p.type === "drop") {
          dragging = false;
          void speech.transcribeFiles(p.paths);
        }
      })
      .then((s) => (stop = s));
    return () => stop?.();
  });

  async function choose() {
    const picked = await pickFiles({
      multiple: true,
      filters: [{ name: "Audio and video", extensions: EXTENSIONS }],
    });
    const paths = Array.isArray(picked) ? picked : picked ? [picked] : [];
    if (paths.length > 0) await speech.transcribeFiles(paths);
  }

  const entry = (id: number) => speech.history.find((h) => h.id === id);
  const textOf = (id: number) => entry(id)?.post_processed_text || entry(id)?.transcription_text || "";
</script>

<div class="flex flex-col gap-4">
  {#if speech.needsSetup}
    <Alert variant="warning"><DangerTriangleIcon size={18} />
      <AlertDescription>
        Set up a speech model first.
      </AlertDescription>
    </Alert>
  {/if}

  <!-- The window's native drop target is the whole page; this box only shows the invitation. -->
  <div
    class="flex flex-col items-center gap-3 rounded-2xl px-6 py-10 text-center transition-colors duration-150 {dragging
      ? 'bg-primary/10'
      : 'bg-card'}"
  >
    <span class="flex size-10 items-center justify-center rounded-xl {dragging ? 'bg-primary/15 text-primary' : 'bg-muted text-muted-foreground'}">
      <UploadMinimalisticIcon size={20} />
    </span>
    <div>
      <p class="text-sm font-medium">{dragging ? "Release to transcribe" : "Drop audio or video files here"}</p>
      <p class="mt-0.5 text-sm text-muted-foreground">MP3, WAV, M4A, MP4 and more</p>
    </div>
    <Button variant="secondary" onclick={choose} disabled={speech.needsSetup}>Choose files</Button>
  </div>

  {#each speech.fileProblems as problem (problem.name)}
    <Alert variant="destructive">
      <DangerTriangleIcon size={18} />
      <AlertDescription><span class="text-sm font-medium">{problem.name}:</span> {problem.error}</AlertDescription>
    </Alert>
  {/each}

  <div class="flex items-center justify-between gap-6 rounded-2xl bg-card px-4 py-3">
    <div class="min-w-0">
      <div class="text-sm font-medium">Show timestamps</div>
      <div class="text-sm text-muted-foreground text-pretty">
        Each line starts with its time.
      </div>
    </div>
    <Switch
      aria-label="Show timestamps by default"
      checked={speech.timestampsDefault}
      onCheckedChange={(v) => speech.setTimestampsDefault(v)}
    />
  </div>

  {#if speech.fileJobs.length === 0}
    <StateView
      icon={UploadMinimalisticIcon}
      title="No files yet" description="Drop audio or video to transcribe it."
      size="sm"
    />
  {:else}
    <ul class="flex flex-col gap-3">
      {#each speech.fileJobs as id (id)}
        {@const e = entry(id)}
        {@const job = speech.jobs[id]}
        <TranscriptRow status={job?.state === "queued" ? "queued" : job?.state === "running" ? "running" : job?.state === "failed" ? "failed" : textOf(id) ? "done" : "queued"}>
          <TranscriptRowHeader>
            <TranscriptRowTitle>{e?.title ?? "Audio file"}</TranscriptRowTitle>
            <TranscriptRowActions>
              <TranscriptRowStatusBadge />
              {#if job?.state === "queued" || job?.state === "running"}
                <TranscriptRowCancel onclick={() => speech.cancelJob(id)} />
              {:else if job?.state === "failed"}
                <TranscriptRowRetry onclick={() => speech.retryJob(id)} />
              {:else if textOf(id)}
                <TranscriptRowCopy text={textOf(id)} />
              {/if}
            </TranscriptRowActions>
          </TranscriptRowHeader>
          {#if job?.state === "running"}
            <TranscriptRowProgress value={job.percent} label={e?.title ?? "Audio file"} />
          {:else if job?.state === "failed"}
            <TranscriptRowError>{job.error}</TranscriptRowError>
          {:else if textOf(id) && e}
            <TranscriptView entry={e} clamp />
          {/if}
        </TranscriptRow>
      {/each}
    </ul>
  {/if}
</div>
