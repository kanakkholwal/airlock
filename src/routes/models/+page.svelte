<script lang="ts">
  import { open as pickFile } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { MemoryCheck, ModelStatus } from "../../bindings";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import { llm } from "#lib/llm.svelte.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";

  let memory = $state<Record<string, MemoryCheck | null>>({});

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

  async function importModel() {
    const path = await pickFile({ filters: [{ name: "GGUF model", extensions: ["gguf"] }] });
    if (typeof path === "string") await llm.importFile(path);
  }
</script>

<main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-16">
  <PageHeader title="Models" description="Weights download on your request and run on this machine." />

  {#if llm.error}
    <p class="rounded-xl bg-destructive/10 px-4 py-3 text-sm text-destructive-strong" role="alert">{llm.error}</p>
  {/if}

  <ul class="flex flex-col gap-3">
    {#each llm.models as m (m.id)}
      {@const dl = llm.downloads[m.id]}
      {@const check = memory[m.id]}
      <li class="flex flex-col gap-3 rounded-2xl bg-card p-4">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
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
                <Button size="sm" variant="secondary" onclick={() => llm.stop()}>Unload</Button>
              {:else}
                <Button size="sm" onclick={() => load(m)}>Load</Button>
              {/if}
              <Button size="sm" variant="ghost" onclick={() => llm.remove(m.id)}>Delete</Button>
            {:else}
              <Button size="sm" onclick={() => llm.download(m.id)}>
                {m.file.state === "partial" ? "Resume" : "Download"}
              </Button>
            {/if}
          </div>
        </div>

        {#if dl}
          <Progress
            value={dl.total > 0 ? (dl.downloaded / dl.total) * 100 : 0}
            indeterminate={dl.total === 0 || dl.phase === "verifying"}
            helper={dl.phase === "verifying"
              ? "Verifying"
              : `${mb(dl.downloaded)} of ${mb(dl.total)} · ${(dl.bytesPerSecond / 1_048_576).toFixed(1)} MB/s`}
          />
        {:else if m.file.state === "partial"}
          <p class="text-sm text-muted-foreground">{mb(m.file.downloaded)} downloaded</p>
        {/if}

        {#if check && !check.fits}
          <p class="text-sm text-destructive-strong" role="alert">
            Needs {gb(check.required_bytes)}, {gb(check.free_bytes)} free. Close other apps or pick a smaller model.
          </p>
        {/if}
      </li>
    {:else}
      <li class="rounded-2xl bg-card p-4 text-sm text-muted-foreground">
        The catalog loads once the app is running.
      </li>
    {/each}
  </ul>

  <div>
    <Button variant="secondary" onclick={importModel}>Import a .gguf file</Button>
  </div>
</main>
