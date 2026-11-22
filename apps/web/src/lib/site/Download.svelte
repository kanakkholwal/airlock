<script lang="ts">
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { DownloadMinimalisticIcon } from "@solar-icons/svelte/linear";
  import { ImageTrail } from "#lib/components/animated/image-trail/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { RELEASES } from "./content.js";

  // Move the pointer over the sheet and pictures from Airlock trail behind it. Pointer only, and
  // off with reduced motion; nothing here is needed to download.
  const trail = ["/img/fox.jpg", "/img/wolf.jpg", "/img/lion.jpg", "/img/nook.jpg", "/img/lighthouse.jpg"];
</script>

<section id="download" class="mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="download-title">
  <div class="relative overflow-hidden rounded-2xl bg-primary px-8 py-20 text-primary-foreground shadow-overlay sm:px-16">
    <div class="absolute inset-0" aria-hidden="true"><ImageTrail images={trail} variant="fall" size="default" /></div>
    <div class="pointer-events-none relative mx-auto flex max-w-2xl flex-col items-center gap-6 text-center">
      <h2 id="download-title" class="text-4xl sm:text-5xl"><RevealText text="Try it on your own computer" trigger="view" staggerMs={70} /></h2>
      <p class="text-lg text-primary-foreground">
        Windows first. The download is small, and models are fetched only when you choose one.
      </p>
      <div class="pointer-events-auto">
        <Button size="lg" href={RELEASES} class="gap-2 bg-card text-foreground hover:bg-card/90">
          <DownloadMinimalisticIcon size={18} />Download for Windows
        </Button>
      </div>
      <p class="text-sm text-primary-foreground">Pre-release, not code-signed yet. macOS and Linux builds are on the way.</p>
    </div>
  </div>
</section>
