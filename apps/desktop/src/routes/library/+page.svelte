<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import {
    DangerTriangleIcon,
    DocumentTextIcon,
    FolderWithFilesIcon,
    GalleryIcon,
    LockKeyholeIcon,
    LockKeyholeUnlockedIcon,
    MagicWandIcon,
    MicrophoneIcon,
    NotesIcon,
    TextFieldIcon,
  } from "@solar-icons/svelte/linear";
  import type { Artifact, Job } from "../../bindings";
  import { inTauri } from "#lib/env.js";
  import { formatDate } from "#lib/format.js";
  import { hasText, library, type Kind } from "#lib/library.svelte.js";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import { Dialog, DialogContent, DialogDescription, DialogTitle } from "#lib/components/ui/dialog/index.js";
  import { Input } from "#lib/components/ui/input/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import { Tooltip, TooltipContent, TooltipTrigger } from "#lib/components/ui/tooltip/index.js";

  // Decision (product): the library is the one place that shows everything Airlock keeps for you,
  // and the place where you decide what could ever leave this computer. Everything starts
  // private. Working on an item, such as summarizing it, is a job that shows its progress here and
  // keeps its work if it is stopped.
  const kinds: { value: Kind; label: string }[] = [
    { value: "", label: "All" },
    { value: "recording", label: "Recordings" },
    { value: "transcript", label: "Files" },
    { value: "note", label: "Notes" },
    { value: "summary", label: "Summaries" },
    { value: "image", label: "Images" },
  ];
  const icons = {
    recording: MicrophoneIcon,
    transcript: DocumentTextIcon,
    note: NotesIcon,
    summary: TextFieldIcon,
    image: GalleryIcon,
  } as const;

  let viewing = $state<{ item: Artifact; text: string } | null>(null);
  let viewOpen = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => void library.init());

  function onSearch(value: string) {
    clearTimeout(timer);
    timer = setTimeout(() => void library.setQuery(value), 200);
  }

  async function view(item: Artifact) {
    const text = await library.text(item.id);
    if (text === null) return;
    viewing = { item, text };
    viewOpen = true;
  }

  const stateLabel = (job: Job) =>
    job.state === "failed" ? "Failed" : job.state === "cancelled" ? "Stopped" : job.state === "interrupted" ? "Interrupted" : "";
  const iconFor = (kind: string) => icons[kind as keyof typeof icons] ?? FolderWithFilesIcon;
</script>

<PageScroll>
  <div class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-12">
    <PageHeader title="Library" />

    {#if !inTauri()}
      <StateView icon={FolderWithFilesIcon} title="Open the Airlock app to see your library" />
    {:else if !library.loaded}
      <CardSkeleton count={3} />
    {:else}
      <p class="flex items-center gap-2 text-sm text-muted-foreground">
        <LockKeyholeIcon size={16} class="shrink-0" />
        Items start private. Only shareable ones can reach an online model.
      </p>

      <div class="flex flex-wrap items-center justify-between gap-3">
        <Tabs value={library.kind} onValueChange={(v) => void library.setKind(v as Kind)} variant="segment">
          <TabsList>
            {#each kinds as k (k.value)}<TabsTrigger value={k.value}>{k.label}</TabsTrigger>{/each}
          </TabsList>
        </Tabs>
        <Input class="w-56" placeholder="Search" aria-label="Search the library" oninput={(e) => onSearch(e.currentTarget.value)} />
      </div>

      {#if library.error}
        <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{library.error}</AlertDescription></Alert>
      {/if}

      {#if library.items.length === 0}
        <StateView
          icon={FolderWithFilesIcon}
          title={library.query || library.kind ? "Nothing matches" : "Your library is empty"}
          description={library.query || library.kind
            ? "Try a different word or another tab."
            : "Dictate something, transcribe a file, make an image or let the assistant save a note. It shows up here."}
        />
      {:else}
        <ul class="flex flex-col gap-3">
          {#each library.items as item (item.id)}
            {@const Icon = iconFor(item.kind)}
            {@const job = library.jobFor(item.id)}
            {@const stopped = library.lastJobFor(item.id)}
            <li class="flex flex-col gap-3 rounded-2xl bg-card p-4 transition-colors duration-150 hover:bg-muted/60 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
              <div class="flex items-start gap-3">
                {#if item.kind === "image" && item.path}
                  <img src={convertFileSrc(item.path)} alt="" class="size-12 shrink-0 rounded-lg object-cover" loading="lazy" />
                {:else}
                  <span class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground">
                    <Icon size={18} />
                  </span>
                {/if}
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium">{item.title}</p>
                  <p class="line-clamp-2 text-sm text-muted-foreground text-pretty">{item.preview}</p>
                  <p class="mt-1 text-xs text-muted-foreground tabular-nums">{formatDate(item.created_ms)} · {item.source}</p>
                </div>
                <Tooltip>
                  <TooltipTrigger>
                    {#snippet child({ props })}
                      <Button
                        {...props}
                        variant="ghost"
                        size="sm"
                        class="shrink-0 gap-1.5 {item.privacy === 'shareable' ? 'text-warning-strong' : 'text-muted-foreground'}"
                        aria-pressed={item.privacy === "shareable"}
                        onclick={() => library.setPrivacy(item.id, item.privacy === "shareable" ? "local_only" : "shareable")}
                      >
                        {#if item.privacy === "shareable"}<LockKeyholeUnlockedIcon size={16} />Shareable{:else}<LockKeyholeIcon size={16} />Private{/if}
                      </Button>
                    {/snippet}
                  </TooltipTrigger>
                  <TooltipContent class="max-w-64 text-pretty">
                    {item.privacy === "shareable" ? "Click to make private." : "Click to make shareable."}
                  </TooltipContent>
                </Tooltip>
              </div>

              {#if job}
                <div class="flex flex-col gap-1.5 pl-12">
                  <Progress value={job.percent} indeterminate={job.state === "queued"} size="sm" label={job.step} />
                  <div class="flex items-center justify-between gap-3 text-sm text-muted-foreground">
                    <span>{job.step}</span>
                    <Button size="sm" variant="ghost" onclick={() => library.cancel(job.id)}>Stop</Button>
                  </div>
                </div>
              {:else if stopped}
                <div class="flex items-center justify-between gap-3 pl-12 text-sm">
                  <span class="flex items-center gap-1.5 text-destructive-strong">
                    <DangerTriangleIcon size={14} />{stateLabel(stopped)}{stopped.error ? `: ${stopped.error}` : ""}
                  </span>
                  <span class="flex gap-1">
                    <Button size="sm" variant="secondary" onclick={() => library.retry(stopped.id)}>Try again</Button>
                    <Button size="sm" variant="ghost" onclick={() => library.clearJob(stopped.id)}>Dismiss</Button>
                  </span>
                </div>
              {:else if hasText(item.kind)}
                <div class="flex gap-2 pl-12">
                  <Button size="sm" variant="secondary" class="gap-1.5" onclick={() => library.summarize(item.id)}>
                    <MagicWandIcon size={16} />Summarize
                  </Button>
                  {#if item.kind === "summary" || item.kind === "note"}
                    <Button size="sm" variant="ghost" onclick={() => view(item)}>Open</Button>
                  {/if}
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
</PageScroll>

<Dialog bind:open={viewOpen} size="lg">
  <DialogContent>
    {#if viewing}
      <DialogTitle>{viewing.item.title}</DialogTitle>
      <DialogDescription>{viewing.item.source}</DialogDescription>
      <div class="mt-3 max-h-[60vh] overflow-y-auto rounded-xl bg-muted/60 p-4 text-pretty whitespace-pre-wrap">{viewing.text}</div>
      <div class="mt-3 flex justify-end gap-2">
        <Badge size="sm" variant="secondary">{viewing.item.privacy === "shareable" ? "Shareable" : "Private"}</Badge>
        <CopyButton text={viewing.text} label="Copy" />
      </div>
    {/if}
  </DialogContent>
</Dialog>
