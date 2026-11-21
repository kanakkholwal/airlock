<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { publisherPageUrl } from "#lib/publisher.js";

  // Opens the publisher's Hugging Face page (an organization or a person) in the browser.
  let { name, class: className = "" }: { name: string; class?: string } = $props();
</script>

{#if name}
  <button
    type="button"
    class="w-fit rounded text-xs text-muted-foreground transition-colors hover:text-foreground hover:underline hover:underline-offset-2 focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none {className}"
    title={`Open ${name} on Hugging Face`}
    onclick={(e) => {
      e.stopPropagation();
      void openUrl(publisherPageUrl(name));
    }}
  >
    by {name}
  </button>
{/if}
