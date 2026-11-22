<script lang="ts">
  import { Button } from "#lib/components/ui/button/index.js";
  import Buddies from "./Buddies.svelte";
  import { REPO } from "./content.js";

  // Decision (product): the reader who worries about privacy will not take our word for it, so the
  // answer is not a claim but an invitation. One headline, one button.
</script>

<section class="relative mx-auto w-full max-w-4xl px-6 py-28 text-center" aria-labelledby="oss-title">
  <Buddies section="open-source" spots={["absolute top-20 left-2 hidden w-20 sm:block", "absolute top-28 right-2 hidden w-20 sm:block"]} />
  <h2 id="oss-title" class="text-4xl sm:text-5xl">Don't trust us. Read the code</h2>
  <div class="mt-8 flex justify-center">
    <Button variant="secondary" href={REPO} target="_blank" rel="noopener noreferrer" class="rounded-full! px-6">View it on GitHub</Button>
  </div>
  <p class="mt-6 text-muted-foreground">No account. Works offline. Every outgoing request is logged.</p>
</section>
