<script lang="ts">
  import { Marker } from "#lib/components/text/marker/index.js";
  import chat from "@airlock/catalog/chat-models";

  // Read from the same catalog the app ships with, so the list here is always the real one.
  const gb = (bytes: number) => `${(bytes / 1_073_741_824).toFixed(1)} GB`;
  const models = chat.items.map((m) => ({
    id: m.id,
    name: m.name,
    description: m.description,
    size: gb(m.model.size_bytes + (m.projector?.size_bytes ?? 0)),
    license: m.license,
    vision: (m.tags as string[]).includes("vision"),
    recommended: m.recommended,
  }));
</script>

<section id="models" class="mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="models-title">
  <div class="grid gap-12 lg:grid-cols-[0.8fr_1.2fr]">
    <div>
      <h2 id="models-title" class="text-4xl sm:text-5xl">Start with a <Marker variant="wavy" tone="primary">small</Marker> model</h2>
      <p class="mt-4 max-w-md text-lg text-muted-foreground">
        Airlock starts with a short list that runs on modest hardware, and can add any GGUF model from Hugging Face. The license is shown before you download.
      </p>
    </div>

    <ul class="flex flex-col gap-3">
      {#each models as m (m.id)}
        <li class="rounded-2xl bg-card p-5 shadow-surface">
          <div class="flex items-baseline justify-between gap-4">
            <h3 class="text-lg">{m.name}</h3>
            <span class="text-sm text-muted-foreground tabular-nums">{m.size}</span>
          </div>
          <p class="mt-1.5 text-muted-foreground">{m.description}</p>
          <p class="mt-3 text-sm text-muted-foreground">
            {m.license}{m.vision ? " · Reads images" : ""}{m.recommended ? " · Recommended" : ""}
          </p>
        </li>
      {/each}
    </ul>
  </div>
</section>
