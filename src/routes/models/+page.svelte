<script lang="ts">
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import { open as pickFile } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Backend, MemoryCheck, ModelStatus } from "../../bindings";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import ExploreModels from "#lib/components/app/explore-models.svelte";
  import SpeechModels from "#lib/components/app/speech-models.svelte";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import { ChatLineIcon, CpuBoltIcon, DangerTriangleIcon, EyeIcon, MonitorIcon } from "@solar-icons/svelte/linear";
  import { inTauri } from "#lib/env.js";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { llm } from "#lib/llm.svelte.js";
  import { rateSpeed, speedLabel, type SpeedRating } from "#lib/speed.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
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

  const loaded = (m: ModelStatus) => {
    const e = llm.engine;
    return (e.state === "ready" || e.state === "loading") && e.model === m.id;
  };

  async function load(m: ModelStatus) {
    const check = await llm.memory(m.id);
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
  <PageHeader title="Models" description="What Airlock can run on this computer. Everything stays here." />

  {#if llm.error}
    <Alert variant="destructive" dismissible><AlertDescription>{llm.error}</AlertDescription></Alert>
  {/if}

  <Tabs bind:value={() => tab, setTab} variant="segment" class="self-center">
    <TabsList>
      <TabsTrigger value="chat">Chat</TabsTrigger>
      <TabsTrigger value="speech">Speech</TabsTrigger>
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
  {:else if tab === "explore"}
    <ExploreModels />
  {:else}
  {#if !inTauri()}
    <StateView
      icon={MonitorIcon}
      title="Open the Airlock app to manage models"
      description="This is a browser preview. Engines and models are downloaded and run by the desktop app."
      variant="card"
    />
  {/if}

  {#if llm.engine.state === "failed"}
    <Alert variant="destructive">
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
        title="No engine for this system"
        description="Airlock does not ship an engine build for this operating system and processor yet."
        variant="card"
      />
    {/if}
    {#if noEngine}
      <p class="px-1 text-sm text-muted-foreground text-pretty">
        An engine runs the models on your hardware. Install one to load a model.
      </p>
    {/if}
    <ul class="flex flex-col gap-2">
      {#each llm.engines as e (e.backend)}
        {@const dl = llm.downloads[`engine:${e.backend}`]}
        <li class="flex flex-col gap-3 rounded-2xl bg-card p-4 transition-colors duration-150 hover:bg-muted/60">
          <div class="flex items-center justify-between gap-4">
            <div class="min-w-0">
              <div class="flex flex-wrap items-center gap-2">
                <CpuBoltIcon size={16} class="shrink-0 text-muted-foreground" />
                <span class="font-medium">{e.label}</span>
                {#if e.recommended}<Badge size="sm">Recommended</Badge>{/if}
                {#if e.installed}<Badge size="sm" variant="success">Installed</Badge>{/if}
              </div>
              <p class="mt-1 text-sm text-muted-foreground">{mb(e.size_bytes)}</p>
            </div>
            <div class="flex shrink-0 gap-2">
              {#if dl}
                <Button size="sm" variant="secondary" onclick={() => llm.cancelEngineInstall(e.backend as Backend)}>Cancel</Button>
              {:else if e.installed}
                {#if confirming === `engine:${e.backend}`}
                  <Button size="sm" variant="destructive" onclick={() => { confirming = null; void llm.removeEngine(e.backend as Backend); }}>
                    Remove {mb(e.size_bytes)}
                  </Button>
                  <Button size="sm" variant="ghost" onclick={() => (confirming = null)}>Keep</Button>
                {:else}
                  <Button size="sm" variant="ghost" onclick={() => (confirming = `engine:${e.backend}`)}>Remove</Button>
                {/if}
              {:else}
                <Button size="sm" variant={e.recommended ? "default" : "secondary"} onclick={() => llm.installEngine(e.backend as Backend)}>Install</Button>
              {/if}
            </div>
          </div>
          {#if dl}
            <DownloadProgress compact title="Download" downloaded={dl.downloaded} total={dl.total} bytesPerSecond={dl.bytesPerSecond} phase={dl.phase} />
          {/if}
        </li>
      {/each}
    </ul>
  </section>

  <section class="flex flex-col gap-2">
    <h2 class="px-1 text-sm font-medium text-muted-foreground">Models</h2>
  <ul class="flex flex-col gap-3">
    {#each llm.models as m (m.id)}
      {@const dl = llm.downloads[m.id]}
      {@const check = memory[m.id]}
      <li class="flex flex-col gap-3 rounded-2xl bg-card p-4 transition-colors duration-150 hover:bg-muted/60">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              {#if m.vision}<EyeIcon size={16} class="shrink-0 text-muted-foreground" />{:else}<ChatLineIcon size={16} class="shrink-0 text-muted-foreground" />{/if}
              <span class="font-medium">{m.name}</span>
              {#if m.recommended}<Badge size="sm">Recommended</Badge>{/if}
              {#if m.vision}<Badge size="sm" variant="info">Vision</Badge>{/if}
              {#if loaded(m)}<Badge size="sm" variant="success">Loaded</Badge>{/if}
            </div>
            <p class="mt-1 text-sm text-muted-foreground text-pretty">{m.description}</p>
            <p class="mt-1 text-sm text-muted-foreground">
              {gb(m.size_bytes)} ·
              {#if m.license_url}
                <button class="underline underline-offset-2" onclick={() => openUrl(m.license_url!)}>
                  {m.license}
                </button>
              {:else}
                {m.license}
              {/if}
            </p>
          </div>

          <div class="flex shrink-0 gap-2">
            {#if dl}
              <Button size="sm" variant="secondary" onclick={() => llm.cancelDownload(m.id)}>Cancel</Button>
            {:else if m.file.state === "downloaded"}
              {#if loaded(m)}
                {#if llm.engine.state === "ready"}
                  {@const s = speed[m.id]}
                  {#if s && s !== "running" && s !== "failed"}
                    <Badge variant={tone[s.rating]}>{s.value.toFixed(0)} tok/s · {speedLabel[s.rating]}</Badge>
                  {:else if s === "failed"}
                    <Badge variant="destructive">Test failed</Badge>
                  {/if}
                  <Button size="sm" variant="ghost" disabled={s === "running"} onclick={() => testSpeed(m.id)}>
                    {s === "running" ? "Testing" : "Test speed"}
                  </Button>
                {/if}
                <Button size="sm" variant="secondary" onclick={() => llm.stop()}>Unload</Button>
              {:else}
                <Button size="sm" variant={llm.current ? "secondary" : m.recommended ? "default" : "secondary"} onclick={() => load(m)}>Load</Button>
              {/if}
              {#if confirming === m.id}
                <Button size="sm" variant="destructive" onclick={() => { confirming = null; void llm.remove(m.id); }}>
                  Delete {gb(m.size_bytes)}
                </Button>
                <Button size="sm" variant="ghost" onclick={() => (confirming = null)}>Keep</Button>
              {:else}
                <Button size="sm" variant="ghost" onclick={() => (confirming = m.id)}>Delete</Button>
              {/if}
            {:else}
              <Button size="sm" variant={m.recommended ? "default" : "secondary"} onclick={() => llm.download(m.id)}>
                {m.file.state === "partial" ? "Resume" : "Download"}
              </Button>
            {/if}
          </div>
        </div>

        {#if dl}
          <DownloadProgress compact title="Download" downloaded={dl.downloaded} total={dl.total} bytesPerSecond={dl.bytesPerSecond} phase={dl.phase} />
        {:else if m.file.state === "partial"}
          <p class="text-sm text-muted-foreground">{mb(m.file.downloaded)} downloaded</p>
        {/if}

        {#if check && !check.fits}
          <Alert variant="warning">
            <AlertDescription>
              Needs {gb(check.required_bytes)} of memory, {gb(check.free_bytes)} free. Close other apps or pick a smaller model.
            </AlertDescription>
          </Alert>
        {/if}
      </li>
    {/each}
  </ul>
  {#if !llm.loaded && inTauri()}
    <CardSkeleton count={2} />
  {:else if llm.loaded && llm.models.length === 0}
    <StateView
      icon={DangerTriangleIcon}
      tone="warning"
      title="The model list did not load"
      description="Airlock could not read its catalog. Reload the screen, or import a .gguf file below."
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
