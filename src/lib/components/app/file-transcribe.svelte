<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open as pickFiles } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import { DangerTriangleIcon, UploadMinimalisticIcon } from "@solar-icons/svelte/linear";
  import { inTauri } from "#lib/env.js";
  import { speech } from "#lib/speech.svelte.js";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";

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
    <Alert variant="warning">
      <AlertDescription>
        Transcribing files needs a speech model. Set it up on the Setup tab first.
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
      <p class="font-[450]">{dragging ? "Release to transcribe" : "Drop audio or video files here"}</p>
      <p class="mt-0.5 text-sm text-muted-foreground">MP3, WAV, M4A, FLAC, OGG, OPUS, MP4 and more. Nothing leaves this computer.</p>
    </div>
    <Button variant="secondary" onclick={choose} disabled={speech.needsSetup}>Choose files</Button>
  </div>

  {#each speech.fileProblems as problem (problem.name)}
    <Alert variant="destructive">
      <DangerTriangleIcon size={18} />
      <AlertDescription><span class="font-[450]">{problem.name}:</span> {problem.error}</AlertDescription>
    </Alert>
  {/each}

  {#if speech.fileJobs.length === 0}
    <StateView
      icon={UploadMinimalisticIcon}
      title="No files yet"
      description="Files you add show up here with their progress, and their text is saved to History."
      size="sm"
    />
  {:else}
    <ul class="flex flex-col gap-3">
      {#each speech.fileJobs as id (id)}
        {@const e = entry(id)}
        {@const job = speech.jobs[id]}
        <li class="flex flex-col gap-2.5 rounded-2xl bg-card p-4">
          <div class="flex items-center justify-between gap-3">
            <span class="min-w-0 truncate font-[450]">{e?.title ?? "Audio file"}</span>
            <div class="flex shrink-0 items-center gap-1.5">
              {#if job?.state === "queued"}
                <Badge size="sm" variant="secondary">Waiting</Badge>
                <Button size="sm" variant="ghost" onclick={() => speech.cancelJob(id)}>Cancel</Button>
              {:else if job?.state === "running"}
                <Badge size="sm" variant="info">Transcribing</Badge>
                <Button size="sm" variant="ghost" onclick={() => speech.cancelJob(id)}>Cancel</Button>
              {:else if job?.state === "failed"}
                <Badge size="sm" variant="destructive">Failed</Badge>
                <Button size="sm" variant="secondary" onclick={() => speech.retryJob(id)}>Try again</Button>
              {:else if textOf(id)}
                <Badge size="sm" variant="success">Done</Badge>
                <CopyButton text={textOf(id)} iconOnly label="Copy text" />
              {/if}
            </div>
          </div>
          {#if job?.state === "running"}
            <Progress value={job.percent} helper={`${Math.round(job.percent)}%`} />
          {:else if job?.state === "failed"}
            <p class="text-sm text-destructive-strong">{job.error}</p>
          {:else if textOf(id)}
            <p class="line-clamp-4 text-sm text-pretty whitespace-pre-wrap text-muted-foreground">{textOf(id)}</p>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>
