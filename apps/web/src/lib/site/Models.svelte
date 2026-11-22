<script lang="ts">
  import chat from "@airlock/catalog/chat-models";
  import images from "@airlock/catalog/image-models";
  import voices from "@airlock/catalog/voices";
  import { Marker } from "#lib/components/text/marker/index.js";
  import Buddies from "./Buddies.svelte";

  // Decision (product): this answers one question, "what do I get without setting anything up?".
  // So it is a plain list grouped by job: the name, how big the download is, and for chat models
  // two bars. No descriptions or licences here; the app shows those before any download. All of it
  // is read from the catalogs the app ships with, so a new model shows up on its own.
  const GB = 1_073_741_824;
  const size = (bytes: number) => (bytes >= GB ? `${(bytes / GB).toFixed(1)} GB` : `${Math.round(bytes / 1_048_576)} MB`);
  const BARS = 5;

  type Item = { name: string; size?: string; speed?: number; smarts?: number };
  type Group = { job: string; items: Item[] };

  const groups: Group[] = [
    {
      job: "Chat",
      items: chat.items.map((m) => {
        const rated = m as { speed?: number; intelligence?: number };
        return {
          name: m.name,
          size: size(m.model.size_bytes + (m.projector?.size_bytes ?? 0)),
          speed: rated.speed,
          smarts: rated.intelligence,
        };
      }),
    },
    {
      job: "Images",
      items: images.items
        .filter((m) => m.recommended)
        .map((m) => ({ name: m.name, size: size(m.files.reduce((n, f) => n + f.size_bytes, 0)) })),
    },
    {
      job: "Voice",
      items: voices.items.map((v) => ({ name: `${v.name}, ${v.voices.length} voices`, size: size(v.archive.size) })),
    },
    { job: "Dictation", items: [{ name: "Parakeet speech to text" }] },
  ];
</script>

{#snippet bars(label: string, value: number)}
  <span class="flex items-center gap-[3px]" role="img" aria-label={`${label} ${value} of ${BARS}`}>
    {#each Array(BARS) as _, i (i)}
      <span class="h-3 w-[3px] rounded-full bg-foreground {i < value ? 'opacity-80' : 'opacity-15'}"></span>
    {/each}
  </span>
{/snippet}

<section id="models" class="relative mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="models-title">
  <Buddies section="models" spots={["absolute top-24 right-[2%] hidden w-20 lg:block", "absolute bottom-16 left-[1%] hidden w-20 xl:block"]} />
  <div class="grid gap-12 lg:grid-cols-[0.8fr_1.2fr] lg:gap-16">
    <div>
      <h2 id="models-title" class="text-4xl sm:text-5xl">Ready in the <Marker variant="wavy" tone="primary">box</Marker></h2>
      <p class="mt-4 text-lg text-muted-foreground">Add any other model from Hugging Face.</p>
    </div>

    <div class="flex flex-col gap-8">
      {#each groups as g (g.job)}
        <div>
          <h3 class="px-4 pb-2 text-sm text-muted-foreground">{g.job}</h3>
          <ul class="rounded-2xl bg-card p-1.5 shadow-(--surface-shadow)">
            {#each g.items as item (item.name)}
              <li class="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 rounded-xl px-3.5 py-3 transition-colors duration-150 hover:bg-muted/60">
                <span class="font-medium">{item.name}</span>
                <span class="flex items-center gap-6 text-sm text-muted-foreground">
                  {#if item.speed !== undefined}
                    <span class="flex items-center gap-2"><span class="hidden sm:inline">Speed</span>{@render bars("Speed", item.speed)}</span>
                  {/if}
                  {#if item.smarts !== undefined}
                    <span class="flex items-center gap-2"><span class="hidden sm:inline">Smarts</span>{@render bars("Smarts", item.smarts)}</span>
                  {/if}
                  {#if item.size}<span class="w-16 text-end tabular-nums">{item.size}</span>{/if}
                </span>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  </div>
</section>
