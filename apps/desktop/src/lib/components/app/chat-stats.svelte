<script lang="ts">
  import {
    ChartSquareIcon,
    ChatLineIcon,
    ClockCircleIcon,
    DocumentTextIcon,
    SpeedometerLowIcon,
  } from "@solar-icons/svelte/linear";
  import type { Component } from "svelte";
  import { chat } from "#lib/chat.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import {
    ContextMeter,
    ContextMeterBar,
    ContextMeterBody,
    ContextMeterHeader,
    ContextMeterIcon,
    ContextMeterPercent,
    ContextMeterSummary,
    ContextMeterTitle,
    formatTokens,
    type ContextSegment,
  } from "#lib/components/agents/context-meter/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Popover, PopoverContent, PopoverTrigger } from "#lib/components/ui/popover/index.js";

  // Decision (product): the context meter answers "will this chat keep working?" and the speed
  // numbers answer "is it fast enough?". The trigger is a bare percentage until the chat gets
  // long. Every number shows a dash until it has been measured.
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

  const dash = "-";
  const tiles = $derived<{ icon: Component<{ size?: number | string; class?: string }>; label: string; value: string }[]>([
    { icon: SpeedometerLowIcon, label: "Speed", value: stats.lastTokensPerSecond === null ? dash : `${stats.lastTokensPerSecond.toFixed(0)} tok/s` },
    { icon: SpeedometerLowIcon, label: "Average", value: stats.averageTokensPerSecond === null ? dash : `${stats.averageTokensPerSecond.toFixed(0)} tok/s` },
    { icon: ClockCircleIcon, label: "First token", value: stats.lastFirstTokenMs === null ? dash : `${(stats.lastFirstTokenMs / 1000).toFixed(1)} s` },
    { icon: ChatLineIcon, label: "Replies", value: String(stats.replies) },
    { icon: DocumentTextIcon, label: "Written", value: `${formatTokens(stats.totalCompletionTokens)} tokens` },
  ]);
</script>

<Popover>
  <PopoverTrigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="ghost"
        size="sm"
        aria-label="Context and speed"
        class="gap-1.5 {percent !== null && percent >= 70 ? 'text-warning-strong' : 'text-muted-foreground'}"
      >
        <ChartSquareIcon size={16} />
        <span class="tabular-nums">{percent === null ? "Context" : `${percent}%`}</span>
      </Button>
    {/snippet}
  </PopoverTrigger>
  <PopoverContent align="end" side="bottom" class="w-72">
    <div class="flex flex-col gap-4">
      <ContextMeter variant="plain" {segments} {limit}>
        <ContextMeterHeader>
          <ContextMeterIcon />
          <ContextMeterTitle>Context</ContextMeterTitle>
          <ContextMeterPercent />
        </ContextMeterHeader>
        <ContextMeterBody>
          <ContextMeterSummary />
          <ContextMeterBar />
        </ContextMeterBody>
      </ContextMeter>

      <dl class="grid grid-cols-2 gap-2">
        {#each tiles as t (t.label)}
          <div class="flex flex-col gap-0.5 rounded-xl bg-muted/60 px-3 py-2 {t.label === 'Written' ? 'col-span-2' : ''}">
            <dt class="flex items-center gap-1.5 text-xs text-muted-foreground"><t.icon size={14} />{t.label}</dt>
            <dd class="text-sm font-medium tabular-nums">{t.value}</dd>
          </div>
        {/each}
      </dl>
    </div>
  </PopoverContent>
</Popover>
