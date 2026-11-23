<script lang="ts">
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#lib/components/ui/collapsible/index.js";
  import { NumberInput } from "#lib/components/ui/number-input/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";
  import { llm } from "#lib/llm.svelte.js";
  import type { Tuning } from "../../../bindings";

  type NumberKey = "gpu_layers" | "context" | "threads" | "batch" | "experts_on_cpu" | "expert_cache_mib";
  // `start` is the value shown when a user switches a setting from Auto to Custom.
  const numbers: { key: NumberKey; title: string; hint: string; min: number; max: number; step: number; start: number; unit?: string }[] = [
    { key: "gpu_layers", title: "GPU layers", hint: "How much of the model runs on the GPU.", min: 0, max: 999, step: 1, start: 20 },
    { key: "context", title: "Chat length", hint: "Auto uses the longest that fits memory.", min: 512, max: 1048576, step: 1024, start: 8192, unit: "tokens" },
    { key: "threads", title: "CPU threads", hint: "Processor cores to use.", min: 1, max: 256, step: 1, start: 8 },
    { key: "batch", title: "Prompt batch", hint: "Bigger reads long prompts faster.", min: 32, max: 8192, step: 128, start: 512 },
    { key: "experts_on_cpu", title: "Experts in RAM", hint: "Expert models only. Layers kept in RAM.", min: 0, max: 999, step: 1, start: 20 },
    { key: "expert_cache_mib", title: "Expert cache", hint: "Expert models only. GPU memory for busy experts.", min: 0, max: 1048576, step: 256, start: 1024, unit: "MiB" },
  ];
  const switches: { key: "flash_attn" | "kv_q8"; title: string; hint: string }[] = [
    { key: "flash_attn", title: "Flash attention", hint: "Faster, lighter long chats." },
    { key: "kv_q8", title: "Compact chat memory", hint: "Half the memory. Needs flash attention." },
  ];

  const t = $derived(llm.tuning);

  function setMode(n: (typeof numbers)[number], mode: string) {
    if (mode === "auto") void llm.setTuning({ [n.key]: null } as Partial<Tuning>);
    else if (mode === "custom" && t[n.key] === null) void llm.setTuning({ [n.key]: n.start } as Partial<Tuning>);
  }
  function setValue(key: NumberKey, v: number | null) {
    if (v !== null && v !== t[key]) void llm.setTuning({ [key]: Math.round(v) } as Partial<Tuning>);
  }
</script>

<Collapsible class="rounded-2xl bg-card px-3">
  <CollapsibleTrigger class="py-3.5">
    <span class="min-w-0 flex-1 text-left">
      <span class="block text-sm font-medium">Advanced</span>
      <span class="block text-xs font-normal text-muted-foreground">Fine-tune how models run.</span>
    </span>
  </CollapsibleTrigger>
  <CollapsibleContent>
    <div class="pb-2">
      {#each numbers as n (n.key)}
        {@const custom = t[n.key] !== null}
        <SettingRow title={n.title} description={n.hint}>
          <span class="flex items-center gap-3">
            {#if custom}
              <NumberInput
                size="sm"
                class="w-36"
                aria-label={n.title}
                value={t[n.key]}
                min={n.min}
                max={n.max}
                step={n.step}
                largeStep={n.step * 10}
                suffix={n.unit ? ` ${n.unit}` : undefined}
                onValueChange={(v) => setValue(n.key, v)}
              />
            {/if}
            <ToggleGroup label={n.title} value={custom ? "custom" : "auto"} onValueChange={(v) => typeof v === "string" && setMode(n, v)}>
              <ToggleGroupItem value="auto">Auto</ToggleGroupItem>
              <ToggleGroupItem value="custom">Custom</ToggleGroupItem>
            </ToggleGroup>
          </span>
        </SettingRow>
      {/each}
      {#each switches as s (s.key)}
        <SettingRow title={s.title} description={s.hint}>
          <Switch
            aria-label={s.title}
            checked={t[s.key]}
            disabled={s.key === "kv_q8" && !t.flash_attn}
            onCheckedChange={(v) => llm.setTuning({ [s.key]: v })}
          />
        </SettingRow>
      {/each}
      <div class="flex items-center justify-between gap-3 border-t border-border/60 px-0 py-3">
        <span class="text-xs text-muted-foreground">Applies on the next model load.</span>
        <Button size="sm" variant="ghost" onclick={() => llm.resetTuning()}>Reset to defaults</Button>
      </div>
    </div>
  </CollapsibleContent>
</Collapsible>
