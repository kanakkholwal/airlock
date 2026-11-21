<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { DangerTriangleIcon, GhostIcon } from "@solar-icons/svelte/linear";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Button } from "#lib/components/ui/button/index.js";
  import { unlockPage } from "#lib/unlock-ui.js";
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";

  // A crash behind an open dialog or sheet leaves the page locked; free it.
  onMount(unlockPage);

  const missing = $derived(page.status === 404);
  const details = $derived(`${page.status} ${page.error?.message ?? ""}`.trim());
</script>

<main class="flex h-full items-center justify-center px-6">
  <div class="w-full max-w-md">
    <StateView
      icon={missing ? GhostIcon : DangerTriangleIcon}
      tone={missing ? "neutral" : "destructive"}
      title={missing ? "That page does not exist" : "This screen hit a problem"}
      description={missing
        ? "The link may be old. Your chats and models are untouched."
        : "Nothing was lost: chats are saved as you go. Try again, or go back to chat."}
    >
      {#snippet actions()}
        <Button onclick={() => goto("/", { invalidateAll: true })}>Back to chat</Button>
        <Button variant="secondary" onclick={() => location.reload()}>Reload</Button>
      {/snippet}
      {#snippet extra()}
        {#if !missing && page.error?.message}
          <div class="mt-4 flex w-full items-start gap-2 rounded-xl bg-muted p-3 text-start">
            <code class="min-w-0 flex-1 text-xs break-words whitespace-pre-wrap text-muted-foreground">{details}</code>
            <CopyButton text={details} iconOnly label="Copy details" />
          </div>
        {/if}
      {/snippet}
    </StateView>
  </div>
</main>
