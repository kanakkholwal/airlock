<script lang="ts">
  import { open as pickFile } from "@tauri-apps/plugin-dialog";
  import { PlayIcon, UploadMinimalisticIcon } from "@solar-icons/svelte/linear";
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { NumberInput } from "#lib/components/ui/number-input/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Slider } from "#lib/components/ui/slider/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";
  import { Textarea } from "#lib/components/ui/textarea/index.js";
  import { ToggleGroup, ToggleGroupItem } from "#lib/components/ui/toggle-group/index.js";
  import { languageName } from "#lib/languages.js";
  import { voicegen } from "#lib/voicegen.svelte.js";
  import type { ModelStatus } from "../../../bindings";

  let { model }: { model: ModelStatus } = $props();

  const has = (c: string) => model.controls.includes(c);
  const s = $derived(voicegen.settings);
  const ready = $derived(voicegen.loaded === model.id);
  const voiceChoices = $derived(
    model.voices.length > 0
      ? model.voices.map((v) => ({ id: v.id, name: v.name }))
      : voicegen.voices.map((v) => ({ id: v, name: v.replaceAll("_", " ") })),
  );

  type AutoKey = "seed" | "temperature" | "flowSteps" | "cfgScale";
  // `start` is the value shown when a setting moves from Auto to Custom.
  const autos: { key: AutoKey; control: string; title: string; hint: string; min: number; max: number; step: number; start: number }[] = [
    { key: "temperature", control: "temperature", title: "Variation", hint: "Higher is livelier, lower is steadier.", min: 0, max: 2, step: 0.1, start: 0.8 },
    { key: "flowSteps", control: "flow_steps", title: "Quality steps", hint: "More steps sound cleaner, but take longer.", min: 2, max: 32, step: 1, start: 10 },
    { key: "cfgScale", control: "cfg_scale", title: "Voice match", hint: "How closely it follows your description.", min: 0.5, max: 5, step: 0.1, start: 2 },
    { key: "seed", control: "seed", title: "Seed", hint: "Same seed, same take. Auto is random.", min: 0, max: 999999, step: 1, start: 42 },
  ];

  function setAuto(key: AutoKey, start: number, mode: string) {
    voicegen.settings[key] = mode === "custom" ? (voicegen.settings[key] ?? start) : null;
  }

  async function chooseClip() {
    const picked = await pickFile({
      multiple: false,
      filters: [{ name: "Audio", extensions: ["wav", "mp3", "flac", "m4a", "ogg"] }],
    });
    if (typeof picked === "string") voicegen.settings.referencePath = picked;
  }
  const clipName = $derived(s.referencePath?.split(/[\\/]/).pop() ?? "");
</script>

<div class="flex flex-col gap-1 rounded-2xl bg-card px-4 pt-3 pb-4">
  <div class="flex items-center justify-between gap-3 pb-1">
    <div>
      <div class="text-sm font-semibold">{model.name} studio</div>
      <div class="text-xs text-muted-foreground">Type a line, adjust the voice, listen.</div>
    </div>
    {#if ready}
      <Button size="sm" variant="ghost" onclick={() => voicegen.unload()}>Unload</Button>
    {/if}
  </div>

  {#if has("design")}
    <div class="flex flex-col gap-1.5 py-2">
      <label class="text-sm font-medium" for="vg-design">Describe the voice</label>
      <Textarea id="vg-design" rows={2} placeholder="A calm older man, warm and slow" bind:value={voicegen.settings.design} />
      <span class="text-xs text-muted-foreground">Empty uses the default voice.</span>
    </div>
  {/if}

  {#if has("language") && model.languages.length > 1}
    <SettingRow title="Language" description="The language of the text.">
      <Select
        items={model.languages.map((c) => ({ value: c, label: languageName(c) }))}
        bind:value={() => s.language ?? model.languages[0], (v) => (voicegen.settings.language = v as string)}
      >
        <SelectTrigger class="w-44"><span class="truncate">{languageName(s.language ?? model.languages[0])}</span></SelectTrigger>
        <SelectContent align="end">
          {#each model.languages as c (c)}<SelectItem value={c}>{languageName(c)}</SelectItem>{/each}
        </SelectContent>
      </Select>
    </SettingRow>
  {/if}

  {#if has("voice") && voiceChoices.length > 0 && !s.referencePath}
    <SettingRow title="Voice" description="A preset voice.">
      <Select
        items={voiceChoices.map((v) => ({ value: v.id, label: v.name }))}
        bind:value={() => s.voice ?? voiceChoices[0].id, (v) => (voicegen.settings.voice = v as string)}
      >
        <SelectTrigger class="w-44"><span class="truncate">{voiceChoices.find((v) => v.id === (s.voice ?? voiceChoices[0].id))?.name}</span></SelectTrigger>
        <SelectContent align="end">
          {#each voiceChoices as v (v.id)}<SelectItem value={v.id}>{v.name}</SelectItem>{/each}
        </SelectContent>
      </Select>
    </SettingRow>
  {/if}

  {#if has("clone")}
    <SettingRow title="Copy a voice" description="A few seconds of clear speech.">
      <span class="flex items-center gap-2">
        {#if s.referencePath}
          <span class="max-w-40 truncate text-xs text-muted-foreground" title={s.referencePath}>{clipName}</span>
          <Button size="sm" variant="ghost" onclick={() => (voicegen.settings.referencePath = null)}>Remove</Button>
        {:else}
          <Button size="sm" variant="secondary" class="gap-1.5" onclick={chooseClip}><UploadMinimalisticIcon size={16} />Choose clip</Button>
        {/if}
      </span>
    </SettingRow>
    {#if has("clone_text") && s.referencePath}
      <div class="flex flex-col gap-1.5 py-2">
        <label class="text-sm font-medium" for="vg-ref">What the clip says</label>
        <Textarea id="vg-ref" rows={2} placeholder="Optional. Helps the copy sound closer." bind:value={voicegen.settings.referenceText} />
      </div>
    {/if}
  {/if}

  {#if has("speed")}
    <SettingRow title="Speed" description="Speaking pace.">
      <div class="w-44">
        <Slider min={0.5} max={2} step={0.05} showValue formatValue={(v) => `${v.toFixed(2)}x`} label="Speed" bind:value={voicegen.settings.speed} />
      </div>
    </SettingRow>
  {/if}

  {#each autos.filter((a) => has(a.control)) as a (a.key)}
    {@const custom = s[a.key] !== null}
    <SettingRow title={a.title} description={a.hint}>
      <span class="flex items-center gap-3">
        {#if custom}
          <NumberInput
            size="sm"
            class="w-28"
            aria-label={a.title}
            value={s[a.key]}
            min={a.min}
            max={a.max}
            step={a.step}
            onValueChange={(v) => (voicegen.settings[a.key] = v)}
          />
        {/if}
        <ToggleGroup label={a.title} value={custom ? "custom" : "auto"} onValueChange={(v) => typeof v === "string" && setAuto(a.key, a.start, v)}>
          <ToggleGroupItem value="auto">Auto</ToggleGroupItem>
          <ToggleGroupItem value="custom">Custom</ToggleGroupItem>
        </ToggleGroup>
      </span>
    </SettingRow>
  {/each}

  <div class="flex flex-col gap-2 pt-3">
    <Textarea rows={3} placeholder="What should it say?" bind:value={voicegen.text} />
    <div class="flex items-center justify-between gap-3">
      <span class="text-xs text-muted-foreground tabular-nums">
        {#if voicegen.clip}{voicegen.clip.seconds.toFixed(1)} s clip{/if}
      </span>
      <span class="flex items-center gap-2">
        {#if !ready}
          <Button size="sm" disabled={voicegen.loading} onclick={() => voicegen.load(model.id)}>
            {#if voicegen.loading}<Spinner size="sm" label="Loading the voice model" />Loading{:else}Load voice model{/if}
          </Button>
        {:else}
          {#if voicegen.clip}
            <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => voicegen.play()}><PlayIcon size={16} />Replay</Button>
          {/if}
          <Button size="sm" disabled={voicegen.making || !voicegen.text.trim()} onclick={() => voicegen.say()}>
            {#if voicegen.making}<Spinner size="sm" label="Making speech" />Making{:else}Speak{/if}
          </Button>
        {/if}
      </span>
    </div>
  </div>

  {#if voicegen.error}
    <Alert variant="destructive"><AlertDescription class="whitespace-pre-line">{voicegen.error}</AlertDescription></Alert>
  {/if}
</div>
