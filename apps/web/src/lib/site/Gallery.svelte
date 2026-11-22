<script lang="ts">
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { RippleTransition } from "#lib/components/animated/ripple-transition/index.js";
  import { pictures } from "./content.js";

  // Click the picture (or press Enter) and the next one ripples open from where you pressed.
  let index = $state(0);
  const current = $derived(pictures[index]);
</script>

<section id="gallery" class="mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="gallery-title">
  <div class="grid items-end gap-6 lg:grid-cols-[1fr_auto]">
    <div class="max-w-2xl">
      <h2 id="gallery-title" class="text-4xl sm:text-5xl"><RevealText text="Describe it, then watch it appear" trigger="view" staggerMs={70} /></h2>
      <p class="mt-4 text-lg text-muted-foreground">Pictures are made by a model on your graphics card and saved to a private gallery on your computer.</p>
    </div>
    <p class="text-sm text-muted-foreground tabular-nums">{index + 1} of {pictures.length}</p>
  </div>

  <div class="mt-10 grid gap-6 lg:grid-cols-[minmax(0,1.3fr)_minmax(0,0.7fr)] lg:items-end">
    <div class="aspect-[4/3] overflow-hidden rounded-2xl shadow-overlay">
      <RippleTransition images={pictures.map(({ src, alt }) => ({ src, alt }))} bind:value={index} label="Show the next picture" />
    </div>
    <figure class="rounded-2xl bg-card p-6 shadow-surface">
      <figcaption class="text-sm text-muted-foreground">The prompt</figcaption>
      {#key index}
        <p class="mt-2 text-xl leading-snug font-medium text-pretty motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-300">
          {current.prompt}
        </p>
      {/key}
    </figure>
  </div>
</section>
