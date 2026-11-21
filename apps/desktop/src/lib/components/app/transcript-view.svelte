<script lang="ts">
  import { ClockCircleIcon, CopyIcon } from "@solar-icons/svelte/linear";
  import type { HistoryEntry } from "../../../bindings";
  import { speech } from "#lib/speech.svelte.js";
  import { stamp, timestamped, toSrt, toVtt } from "#lib/transcript.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
  } from "#lib/components/ui/dropdown-menu/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";

  // Decision (product): a recording of a podcast or a meeting is read by time, a dictation is
  // read as text. Files that have times offer both views, and the default is a setting, so people
  // who work with timed audio get it first and everyone else never sees it.
  let { entry, clamp = false }: { entry: HistoryEntry; clamp?: boolean } = $props();

  const segments = $derived(entry.segments ?? []);
  const timed = $derived(segments.length > 0);
  const plain = $derived(entry.post_processed_text || entry.transcription_text);
  let override = $state<"text" | "timed" | null>(null);
  const view = $derived(timed ? (override ?? (speech.timestampsDefault ? "timed" : "text")) : "text");
  const hours = $derived((segments.at(-1)?.end_ms ?? 0) >= 3_600_000);

  async function copy(text: string) {
    await navigator.clipboard.writeText(text);
  }
</script>

<div class="flex flex-col gap-2">
  {#if timed}
    <div class="flex items-center justify-between gap-2">
      <ToggleGroup
        type="single"
        variant="segmented"
        size="sm"
        label="Transcript view"
        value={view}
        onValueChange={(v) => {
          if (v === "text" || v === "timed") override = v;
        }}
      >
        <ToggleGroupItem value="text">Text</ToggleGroupItem>
        <ToggleGroupItem value="timed" class="gap-1.5"><ClockCircleIcon size={14} />Timestamps</ToggleGroupItem>
      </ToggleGroup>
      <DropdownMenu>
        <DropdownMenuTrigger>
          {#snippet child({ props })}
            <Button {...props} variant="ghost" size="sm" class="gap-1.5"><CopyIcon size={16} />Copy as</Button>
          {/snippet}
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onclick={() => copy(plain)}>Plain text</DropdownMenuItem>
          <DropdownMenuItem onclick={() => copy(timestamped(segments))}>Text with timestamps</DropdownMenuItem>
          <DropdownMenuItem onclick={() => copy(toSrt(segments))}>Captions (.srt)</DropdownMenuItem>
          <DropdownMenuItem onclick={() => copy(toVtt(segments))}>Captions (.vtt)</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  {/if}

  {#if view === "timed"}
    <ol class="flex flex-col gap-1.5 {clamp ? 'max-h-40 overflow-y-auto' : ''}">
      {#each segments as s, i (i)}
        <li class="flex gap-3 text-pretty">
          <span class="w-14 shrink-0 pt-0.5 text-right font-mono text-xs text-muted-foreground tabular-nums">
            {stamp(s.start_ms, hours)}
          </span>
          <span>{s.text}</span>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="text-pretty whitespace-pre-wrap {clamp ? 'line-clamp-4 text-sm text-muted-foreground' : ''}">{plain}</p>
  {/if}
</div>
