<script lang="ts">
  import { TextReel } from "#lib/components/text/text-reel/index.js";
  import { tools } from "./content.js";

  // A band of the tools that drifts, then speeds up and turns with the page scroll. It is decoration,
  // so the real list lives in the Tools section. Held still under reduced motion.
  const items = tools.map((t) => t.name);
</script>

<div class="overflow-hidden py-10 text-muted-foreground" aria-hidden="true">
  <TextReel orientation="horizontal" {items} size="lg" speed={0.5} />
</div>
