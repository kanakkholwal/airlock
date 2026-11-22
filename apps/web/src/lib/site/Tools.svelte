<script lang="ts">
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { OrbitCardStack } from "#lib/components/animated/orbit-card-stack/index.js";
  import { tools } from "./content.js";

  // The six tools as a pile of cards that fans out when you reach for it. Hover or focus a card
  // to read it; arrow keys move between them. The text is the same data the rest of the page uses.
  let active = $state(0);
  const items = tools.map((t) => ({ name: t.name, role: t.role, description: t.description, initials: t.name.slice(0, 2) }));
</script>

<section id="tools" class="relative mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="tools-title">
  <div class="grid items-center gap-12 lg:grid-cols-[0.8fr_1.2fr]">
    <div>
      <h2 id="tools-title" class="text-4xl sm:text-5xl"><RevealText text="Six tools in one app" trigger="view" staggerMs={80} /></h2>
      <p class="mt-4 max-w-md text-lg text-muted-foreground">
        They share the same models and the same private library, so what you dictate can be summarized, read back, or turned into a picture.
      </p>
      <p class="mt-6 text-sm text-muted-foreground" aria-live="polite">
        <span class="font-medium text-foreground">{tools[active]?.name}</span>: {tools[active]?.description}
      </p>
    </div>
    <OrbitCardStack {items} bind:value={active} layout="arc" size="lg" labels={{ group: "Airlock tools", link: "Open" }} />
  </div>
</section>
