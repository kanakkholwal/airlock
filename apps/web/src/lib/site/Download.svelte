<script lang="ts">
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { DownloadMinimalisticIcon } from "@solar-icons/svelte/linear";
  import { Button } from "#lib/components/ui/button/index.js";
  import { RELEASES } from "./content.js";

</script>

<section id="download" class="mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="download-title">
  <div class="relative overflow-hidden rounded-2xl bg-foreground px-8 py-20 text-background shadow-overlay sm:px-16">
    <div class="relative mx-auto flex max-w-2xl flex-col items-center gap-6 text-center">
      <h2 id="download-title" class="text-4xl sm:text-5xl"><RevealText text="Try it on your own computer" trigger="view" staggerMs={70} /></h2>
      <p class="text-lg text-background/80">
        Windows first. The download is small, and models are fetched only when you choose one.
      </p>
      <div>
        <Button size="lg" href={RELEASES} variant="light" class="gap-2 rounded-full!">
          <DownloadMinimalisticIcon size={18} />Download for Windows
        </Button>
      </div>
      <p class="text-sm text-background/80">Pre-release, not code-signed yet. macOS and Linux builds are on the way.</p>
    </div>
  </div>
</section>
