<script lang="ts">
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { ScrollChoreography } from "#lib/components/animated/scroll-choreography/index.js";

  // Four pictures swap, stack, and one grows to fill the frame as the box scrolls. All are
  // Airlock output, so the proof sits in the motion instead of a claim.
  const images = {
    topLeft: { src: "/img/lighthouse.jpg", alt: "A lighthouse on a cliff at dusk, as an oil painting" },
    topRight: { src: "/img/wolf.jpg", alt: "A paper-craft wolf standing in a forest" },
    bottomLeft: { src: "/img/nook.jpg", alt: "A book on a windowsill with rain on the glass" },
    bottomRight: { src: "/img/lion.jpg", alt: "A paper-craft lion in a forest" },
  };
</script>

<section class="mx-auto w-full max-w-6xl px-6 py-24" aria-labelledby="reveal-title">
  <div class="mb-10 max-w-2xl">
    <h2 id="reveal-title" class="text-4xl sm:text-5xl"><RevealText text="Every picture here was made on one laptop" trigger="view" staggerMs={70} /></h2>
    <p class="mt-4 text-lg text-muted-foreground">A 4 GB graphics card, no account, and no upload. Scroll inside the frame to watch them settle.</p>
  </div>
  <div class="overflow-hidden rounded-2xl bg-card shadow-overlay">
    <ScrollChoreography {images} size="default" label="Scroll to arrange the pictures" />
  </div>
</section>
