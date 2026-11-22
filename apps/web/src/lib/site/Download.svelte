<script lang="ts">
  import { DownloadMinimalisticIcon } from "@solar-icons/svelte/linear";
  import {
    CtaBanner,
    CtaBannerActions,
    CtaBannerContent,
    CtaBannerGrain,
    CtaBannerNote,
    CtaBannerTitle,
  } from "#lib/components/blocks/cta-banner/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import Buddies from "./Buddies.svelte";
  import { RELEASES } from "./content.js";

  // The closing banner from Baby UI, dressed like the hero: the same soft tinted panel, orange glow
  // from the top, vignette and film grain, with dark text. The component is built for a coloured
  // panel with white text, so the text and surface colours are set here. Four characters drift in
  // the corners.
</script>

<section id="download" class="mx-auto w-full max-w-6xl scroll-mt-24 px-6 py-24" aria-labelledby="download-title">
  <CtaBanner variant="grain" tint="var(--muted)" class="bg-muted/30! text-foreground!">
    <span aria-hidden="true" class="pointer-events-none absolute top-0 left-1/3 size-1/2 -translate-1/2 rounded-full bg-linear-to-b from-primary/70 to-transparent blur-3xl"></span>
    <span aria-hidden="true" class="pointer-events-none absolute top-0 left-2/3 size-1/3 -translate-1/2 rounded-full bg-linear-to-b from-warning/50 to-transparent blur-3xl"></span>
    <span
      aria-hidden="true"
      class="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_-60%,transparent_0%,color-mix(in_oklch,var(--muted)_50%,transparent)_50%,color-mix(in_oklch,var(--background)_80%,transparent)_100%)]"
    ></span>
    <CtaBannerGrain noise={0.5} lines={false} />

    <Buddies section="download" spots={[
      "absolute top-[18%] left-[6%] hidden w-20 sm:block",
      "absolute top-[14%] right-[7%] hidden w-20 sm:block",
      "absolute bottom-[14%] left-[10%] hidden w-20 sm:block",
      "absolute right-[9%] bottom-[16%] hidden w-20 sm:block",
    ]} />

    <CtaBannerContent>
      <CtaBannerTitle id="download-title" class="text-foreground!">Free. A few minutes to set up</CtaBannerTitle>
      <CtaBannerActions>
        <Button size="lg" href={RELEASES} variant="dark" class="gap-2 rounded-full!">
          <DownloadMinimalisticIcon size={18} />Download for Windows
        </Button>
      </CtaBannerActions>
      <CtaBannerNote class="text-muted-foreground!">Windows first. macOS and Linux are coming.</CtaBannerNote>
    </CtaBannerContent>
  </CtaBanner>
</section>
