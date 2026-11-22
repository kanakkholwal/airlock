<script lang="ts">
  import { Marker } from "#lib/components/text/marker/index.js";
  import Buddies from "./Buddies.svelte";
  import TornEdge from "./TornEdge.svelte";
  import { ledger } from "./content.js";

  // Decision (product): privacy is a list you can check. These are all the ways anything leaves
  // the computer, with when and where to. It is the same list as Settings, Privacy in the app.
</script>

<section id="privacy" class="relative scroll-mt-24" aria-labelledby="privacy-title">
  <Buddies section="privacy" spots={["absolute top-28 right-[3%] hidden w-20 lg:block", "absolute bottom-24 left-[3%] hidden w-24 lg:block"]} />
  <TornEdge fill="var(--card)" />
  <div class="bg-card">
    <div class="mx-auto grid w-full max-w-6xl gap-12 px-6 py-20 lg:grid-cols-[0.8fr_1.2fr]">
      <div>
        <h2 id="privacy-title" class="text-4xl sm:text-5xl">Private isn't a promise. It's a <Marker variant="circle" tone="primary">list</Marker></h2>
      </div>

      <dl class="flex flex-col">
        {#each ledger as row (row.what)}
          <div class="grid gap-x-6 gap-y-1 border-b border-dashed border-border-strong py-4 first:pt-0 last:border-b-0 sm:grid-cols-[1.1fr_0.8fr_1.2fr] sm:items-baseline">
            <dt class="font-medium">{row.what}</dt>
            <dd class="text-muted-foreground">{row.where}</dd>
            <dd class={row.when === "Never leaves" ? "font-medium text-accent-text" : "text-muted-foreground"}>{row.when}</dd>
          </div>
        {/each}
      </dl>
    </div>
  </div>
  <TornEdge fill="var(--card)" flip />
</section>
