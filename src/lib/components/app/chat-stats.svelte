<script lang="ts">
  import { ChartSquareIcon } from "@solar-icons/svelte/linear";
  import { chat } from "#lib/chat.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import {
    ContextMeter,
    ContextMeterBar,
    ContextMeterBody,
    ContextMeterHeader,
    ContextMeterIcon,
    ContextMeterList,
    ContextMeterPercent,
    ContextMeterSubtitle,
    ContextMeterSummary,
    ContextMeterTitle,
    formatTokens,
    type ContextSegment,
  } from "#lib/components/agents/context-meter/index.js";
  import { Chart2Icon } from "@solar-icons/svelte/linear";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Popover, PopoverContent, PopoverTrigger } from "#lib/components/ui/popover/index.js";

  // Decision (product): the context meter answers "will this chat keep working?", so it shows
  // real server token counts and stays quiet (a plain percentage) until the chat gets long.
  const stats = $derived(chat.stats);
  const limit = $derived(llm.contextTokens);
  const percent = $derived(stats.contextTokens === null ? null : Math.round((stats.contextTokens / limit) * 100));
  const segments = $derived<ContextSegment[]>(
    stats.contextTokens === null
      ? []
      : [
          { id: "conversation", label: "Conversation", value: stats.lastPromptTokens ?? 0 },
          { id: "reply", label: "Last reply", value: stats.lastCompletionTokens ?? 0 },
        ],
  );

  const speed = (v: number | null) => (v === null ? "Not measured yet" : `${v.toFixed(0)} tok/s`);
  const seconds = (ms: number | null) => (ms === null ? "Not measured yet" : `${(ms / 1000).toFixed(1)} s`);
</script>

<Popover>
  <PopoverTrigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="ghost"
        size="sm"
        aria-label="Chat statistics"
        class="gap-1.5 {percent !== null && percent >= 70 ? 'text-warning-strong' : 'text-muted-foreground'}"
      >
        <ChartSquareIcon size={16} />
        <span class="tabular-nums">{percent === null ? "Context" : `${percent}% context`}</span>
      </Button>
    {/snippet}
  </PopoverTrigger>
  <PopoverContent align="end" side="bottom" class="w-80">
    <div class="flex flex-col gap-3">
      <ContextMeter variant="plain" {segments} {limit}>
        <ContextMeterHeader>
          <ContextMeterIcon />
          <ContextMeterTitle>Context window</ContextMeterTitle>
          <ContextMeterSubtitle>{llm.current?.name ?? "No model"}</ContextMeterSubtitle>
          <ContextMeterPercent />
        </ContextMeterHeader>
        <ContextMeterBody>
          <ContextMeterSummary />
          <ContextMeterBar />
          <ContextMeterList />
        </ContextMeterBody>
      </ContextMeter>

      {#if stats.contextTokens === null}
        <div class="flex items-start gap-3 rounded-xl bg-muted p-3">
          <Chart2Icon size={20} class="mt-0.5 shrink-0 text-muted-foreground" />
          <p class="text-sm text-pretty">
            <span class="font-medium">No replies yet.</span>
            <span class="text-muted-foreground">
              This model holds {formatTokens(limit)} tokens. Usage and speed appear after the first reply.
            </span>
          </p>
        </div>
      {/if}

      <dl class="grid grid-cols-[1fr_auto] gap-x-4 gap-y-1.5 text-sm">
        <dt class="text-muted-foreground">Last reply speed</dt>
        <dd class="tabular-nums">{speed(stats.lastTokensPerSecond)}</dd>
        <dt class="text-muted-foreground">Average speed</dt>
        <dd class="tabular-nums">{speed(stats.averageTokensPerSecond)}</dd>
        <dt class="text-muted-foreground">Time to first token</dt>
        <dd class="tabular-nums">{seconds(stats.lastFirstTokenMs)}</dd>
        <dt class="text-muted-foreground">Replies</dt>
        <dd class="tabular-nums">{stats.replies}</dd>
        <dt class="text-muted-foreground">Tokens written</dt>
        <dd class="tabular-nums">{formatTokens(stats.totalCompletionTokens)}</dd>
      </dl>
    </div>
  </PopoverContent>
</Popover>
