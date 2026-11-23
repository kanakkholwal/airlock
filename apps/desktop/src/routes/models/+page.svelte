<script lang="ts">
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import { open as pickFile } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Backend, MemoryCheck, ModelStatus } from "../../bindings";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import ExploreModels from "#lib/components/app/explore-models.svelte";
  import SpeechModels from "#lib/components/app/speech-models.svelte";
  import VoiceGeneration from "#lib/components/app/voice-generation.svelte";
  import VoiceModels from "#lib/components/app/voice-models.svelte";
  import EngineTuning from "#lib/components/app/engine-tuning.svelte";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import { ChatLineIcon, CpuBoltIcon, CpuIcon, DocumentTextIcon, DownloadMinimalisticIcon, DangerTriangleIcon, EyeIcon, MonitorIcon } from "@solar-icons/svelte/linear";
  import { inTauri } from "#lib/env.js";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { llm } from "#lib/llm.svelte.js";
  import { rateSpeed, speedLabel, type SpeedRating } from "#lib/speed.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import ModelCard from "#lib/components/app/model-card.svelte";
  import ModelMenu from "#lib/components/app/model-menu.svelte";
  import { Spinner } from "#lib/components/ui/spinner/index.js";
  import DownloadProgress from "#lib/components/app/download-progress.svelte";

  // The tab lives in the URL, so other screens can link straight to Speech or Explore.
  const tab = $derived(page.url.searchParams.get("tab") ?? "chat");
  const setTab = (v: string) => void goto(`?tab=${v}`, { replace: true });

  let memory = $state<Record<string, MemoryCheck | null>>({});
  // Which delete or remove is waiting for a second click (multi-GB files are not recoverable).
  let confirming = $state<string | null>(null);
  let speed = $state<Record<string, { value: number; rating: SpeedRating } | "running" | "failed">>({});

  async function testSpeed(id: string) {
    const run = llm.speedRunner;
    if (!run) return;
    speed[id] = "running";
    try {
      const [r] = await run("chat");
      speed[id] = { value: r.value, rating: rateSpeed(r.value) };
    } catch {
      speed[id] = "failed";
    }
  }

  const tone: Record<SpeedRating, "success" | "info" | "warning" | "destructive"> = {
    excellent: "success",
    decent: "info",
    usable: "warning",
    slow: "destructive",
  };

  const mb = (b: number) => `${Math.round(b / 1_048_576)} MB`;
  const gb = (b: number) => `${(b / 1_073_741_824).toFixed(1)} GB`;
  const ctx = (n: number) => (n >= 1000 ? `${Math.round(n / 1024)}K` : String(n));

  const loaded = (m: ModelStatus) => {
    const e = llm.engine;
    return (e.state === "ready" || e.state === "loading") && e.model === m.id;
  };

  // Which model is being checked for memory, so its button shows it is working.
  let checking = $state<string | null>(null);

  async function load(m: ModelStatus) {
    checking = m.id;
    const check = await llm.memory(m.id).finally(() => (checking = null));
    memory[m.id] = check;
    // Never attempt a load that cannot fit (plan section 14).
    if (check && !check.fits) return;
    await llm.start(m.id);
  }

  const noEngine = $derived(llm.engines.length > 0 && !llm.engines.some((e) => e.installed));

  async function importModel() {
    const path = await pickFile({ filters: [{ name: "GGUF model", extensions: ["gguf"] }] });
    if (typeof path === "string") await llm.importFile(path);
  }
</script>

<PageScroll>
<main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-16">
  <PageHeader title="Models" />

  {#if llm.error}
    <Alert variant="destructive" dismissible><DangerTriangleIcon size={18} /><AlertDescription>{llm.error}</AlertDescription></Alert>
  {/if}

  <Tabs bind:value={() => tab, setTab} variant="segment" class="self-center">
    <TabsList>
      <TabsTrigger value="chat">Chat</TabsTrigger>
      <TabsTrigger value="speech">Speech</TabsTrigger>
      <TabsTrigger value="voice">Voice</TabsTrigger>
      <TabsTrigger value="explore">Explore</TabsTrigger>
    </TabsList>
  </Tabs>

  {#if tab === "speech"}
    <section class="flex flex-col gap-3">
      <p class="px-1 text-sm text-muted-foreground text-pretty">
        Speech models turn your voice into text for dictation. They run on this computer.
      </p>
      <SpeechModels />
    </section>
  {:else if tab === "voice"}
    <div class="flex flex-col gap-8">
      <VoiceGeneration />
      <section class="flex flex-col gap-3">
        <h2 class="px-1 text-sm font-medium text-muted-foreground">Read aloud</h2>
        <VoiceModels />
      </section>
    </div>
  {:else if tab === "explore"}
    <ExploreModels />
  {:else}
  {#if !inTauri()}
    <StateView
      icon={MonitorIcon}
      title="Open the Airlock app to manage models"
      variant="card"
    />
  {/if}

  {#if llm.engine.state === "failed"}
    <Alert variant="destructive"><DangerTriangleIcon size={18} />
      <AlertDescription>
        The engine stopped: {llm.engine.reason} Pick a model below to try again, or choose a different engine.
      </AlertDescription>
    </Alert>
  {/if}

  <section class="flex flex-col gap-2">
    <h2 class="px-1 text-sm font-medium text-muted-foreground">Engine</h2>
    {#if !llm.loaded && inTauri()}
      <CardSkeleton count={2} />
    {:else if llm.loaded && inTauri() && llm.engines.length === 0}
      <StateView
        icon={CpuBoltIcon}
        tone="warning"
        title="No engine for this system" description="No build for this system yet."
        variant="card"
      />
    {/if}
    {#if noEngine}
      <p class="px-1 text-sm text-muted-foreground text-pretty">
        Install an engine to run models.
      </p>
    {/if}
    {#if llm.engines.filter((e) => e.installed).length > 1}
      <p class="px-1 text-sm text-muted-foreground">
        {#if llm.enginePref === "auto"}
          Automatic. Pick an engine to choose yourself.
        {:else}
          Using {llm.engines.find((e) => e.backend === llm.enginePref)?.label ?? llm.enginePref}.
          <button class="underline underline-offset-2 hover:text-foreground" onclick={() => llm.setEnginePref("auto")}>Use automatic</button>
        {/if}
      </p>
    {/if}
    <ul class="flex flex-col gap-3">
      {#each llm.engines as e (e.backend)}
        {@const dl = llm.downloads[`engine:${e.backend}`]}
        <ModelCard icon={CpuBoltIcon} title={e.label} facts={[mb(e.size_bytes)]} active={e.installed}>
          {#snippet badges()}
            {#if llm.engine.state === "ready" && llm.engine.backend === e.backend}<Badge size="sm" variant="info">Running now</Badge>{/if}
            {#if e.installed && llm.enginePref === e.backend}<Badge size="sm" variant="success">Your choice</Badge>
            {:else if e.installed}<Badge size="sm" variant="success">Installed</Badge>{/if}
            {#if e.recommended}<Badge size="sm">Recommended</Badge>{/if}
          {/snippet}
          {#snippet actions()}
            {#if dl}
              <Button size="sm" variant="secondary" onclick={() => llm.cancelEngineInstall(e.backend as Backend)}>Cancel</Button>
            {:else if e.installed}
              {#if llm.enginePref !== e.backend}
                <Button size="sm" variant="secondary" onclick={() => llm.setEnginePref(e.backend as Backend)}>Use this</Button>
              {/if}
              <ModelMenu items={[{ label: "Remove", onselect: () => (confirming = `engine:${e.backend}`) }]} />
            {:else}
              <Button size="sm" variant={e.recommended ? "default" : "secondary"} onclick={() => llm.installEngine(e.backend as Backend)}>Install</Button>
            {/if}
          {/snippet}
          {#snippet footer()}
            {#if dl}
              <DownloadProgress compact title="Download" downloaded={dl.downloaded} total={dl.total} bytesPerSecond={dl.bytesPerSecond} phase={dl.phase} />
            {:else if confirming === `engine:${e.backend}`}
              <div class="flex items-center justify-between gap-3 rounded-xl bg-muted/60 px-3 py-2 text-sm">
                <span>Remove this engine and free {mb(e.size_bytes)}?</span>
                <span class="flex gap-1.5">
                  <Button size="sm" variant="destructive" onclick={() => { confirming = null; void llm.removeEngine(e.backend as Backend); }}>Remove</Button>
                  <Button size="sm" variant="ghost" onclick={() => (confirming = null)}>Keep</Button>
                </span>
              </div>
            {/if}
          {/snippet}
        </ModelCard>
      {/each}
    </ul>
    <EngineTuning />
  </section>

  <section class="flex flex-col gap-2">
    <h2 class="px-1 text-sm font-medium text-muted-foreground">Models</h2>
  <ul class="flex flex-col gap-3">
    {#each llm.models as m (m.id)}
      {@const dl = llm.downloads[m.id]}
      {@const check = memory[m.id]}
      {@const s = speed[m.id]}
      <ModelCard
        icon={m.vision ? EyeIcon : ChatLineIcon}
        title={m.name}
        description={m.description}
        facts={[m.license]}
        active={loaded(m) && llm.engine.state === "ready"}
      >
        {#snippet specs()}
          <span class="inline-flex items-center gap-1.5" title="Disk space">
            <DownloadMinimalisticIcon size={14} />{gb(m.size_bytes)} disk
          </span>
          {#if m.needs_gb}
            <span class="inline-flex items-center gap-1.5" title="Estimated RAM plus GPU memory at your current settings">
              <CpuIcon size={14} />~{m.needs_gb} GB memory
            </span>
          {/if}
          {#if m.max_context}
            <span class="inline-flex items-center gap-1.5" title="Longest text it can read at once">
              <DocumentTextIcon size={14} />{ctx(m.max_context)} context
            </span>
          {/if}
        {/snippet}
        {#snippet badges()}
          {#if loaded(m)}<Badge size="sm" variant={llm.engine.state === "loading" ? "info" : "success"}>{llm.engine.state === "loading" ? "Loading" : "Loaded"}</Badge>{/if}
          {#if m.recommended}<Badge size="sm">Recommended</Badge>{/if}
          {#if m.vision}<Badge size="sm" variant="info">Vision</Badge>{/if}
          {#if m.tags?.includes("uncensored")}<Badge size="sm" variant="warning">Uncensored</Badge>{/if}
          {#if loaded(m) && llm.engine.state === "ready" && s && s !== "running" && s !== "failed"}
            <Badge size="sm" variant={tone[s.rating]}>{s.value.toFixed(0)} tok/s · {speedLabel[s.rating]}</Badge>
          {:else if s === "failed"}
            <Badge size="sm" variant="destructive">Test failed</Badge>
          {/if}
        {/snippet}
        {#snippet actions()}
          {#if dl}
            <Button size="sm" variant="secondary" onclick={() => llm.cancelDownload(m.id)}>Cancel</Button>
          {:else if m.file.state === "downloaded"}
            {#if loaded(m)}
              <Button size="sm" variant="secondary" onclick={() => llm.stop()}>Unload</Button>
            {:else}
              <Button
                size="sm"
                variant={llm.current ? "secondary" : m.recommended ? "default" : "secondary"}
                disabled={llm.engine.state === "loading" || checking === m.id}
                onclick={() => load(m)}
              >
                {#if checking === m.id}<Spinner size="sm" label="Checking memory" />{/if}
                Load
              </Button>
            {/if}
            <ModelMenu
              items={[
                ...(loaded(m) && llm.engine.state === "ready"
                  ? [{ label: s === "running" ? "Testing speed" : "Test speed", onselect: () => testSpeed(m.id), disabled: s === "running" }]
                  : []),
                ...(m.license_url ? [{ label: "View license", onselect: () => void openUrl(m.license_url!) }] : []),
                { label: "Delete", onselect: () => (confirming = m.id) },
              ]}
            />
          {:else}
            <Button size="sm" variant={m.recommended ? "default" : "secondary"} onclick={() => llm.download(m.id)}>
              {m.file.state === "partial" ? "Resume" : "Download"}
            </Button>
            {#if m.license_url}<ModelMenu items={[{ label: "View license", onselect: () => void openUrl(m.license_url!) }]} />{/if}
          {/if}
        {/snippet}
        {#snippet footer()}
          {#if dl}
            <DownloadProgress compact title="Download" downloaded={dl.downloaded} total={dl.total} bytesPerSecond={dl.bytesPerSecond} phase={dl.phase} />
          {:else if m.file.state === "partial"}
            <p class="text-xs text-muted-foreground">{mb(m.file.downloaded)} downloaded</p>
          {/if}
          {#if confirming === m.id}
            <div class="flex items-center justify-between gap-3 rounded-xl bg-muted/60 px-3 py-2 text-sm">
              <span>Delete this model and free {gb(m.size_bytes)}?</span>
              <span class="flex gap-1.5">
                <Button size="sm" variant="destructive" onclick={() => { confirming = null; void llm.remove(m.id); }}>Delete</Button>
                <Button size="sm" variant="ghost" onclick={() => (confirming = null)}>Keep</Button>
              </span>
            </div>
          {/if}
          {#if check && !check.fits}
            <Alert variant="warning"><DangerTriangleIcon size={18} />
              <AlertDescription>
                Needs {gb(check.required_bytes)} of memory, {gb(check.free_bytes)} free. Close other apps or pick a smaller model.
              </AlertDescription>
            </Alert>
          {/if}
        {/snippet}
      </ModelCard>
    {/each}
  </ul>
  {#if !llm.loaded && inTauri()}
    <CardSkeleton count={2} />
  {:else if llm.loaded && llm.models.length === 0}
    <StateView
      icon={DangerTriangleIcon}
      tone="warning"
      title="The model list did not load" description="Reload, or import a .gguf file below."
      variant="card"
    >
      {#snippet actions()}
        <Button variant="secondary" onclick={() => llm.refresh()}>Try again</Button>
      {/snippet}
    </StateView>
  {/if}

  </section>

  <div>
    <Button variant="secondary" onclick={importModel}>Import a .gguf file</Button>
  </div>

  <p class="px-1 text-sm text-muted-foreground text-pretty">
    Where files are stored is in <a class="underline underline-offset-2" href="/settings">Settings</a>.
  </p>
  {/if}
</main>
</PageScroll>
