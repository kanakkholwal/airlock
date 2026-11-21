<script lang="ts">
  import { DangerTriangleIcon } from "@solar-icons/svelte/linear";
  import { llm } from "#lib/llm.svelte.js";
  import { Alert, AlertDescription, AlertTitle } from "#lib/components/ui/alert/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";

  // Decision (product): loading a model is the slowest thing the app does, so it gets its own
  // visible state with a reason, instead of a disabled input or a silent wait.
  const engine = $derived(llm.engine);
  const name = $derived(llm.current?.name ?? "the model");
</script>

{#if engine.state === "loading"}
  <Alert variant="info">
    <Spinner size="sm" label="Loading model" />
    <AlertTitle>Loading {name}</AlertTitle>
    <AlertDescription>
      Your message sends when it is ready.</AlertDescription>
  </Alert>
{:else if engine.state === "failed"}
  <Alert variant="destructive">
    <DangerTriangleIcon size={18} />
    <AlertTitle>The model stopped</AlertTitle>
    <AlertDescription>
      {engine.reason}
      <span class="mt-2 flex gap-2">
        {#if llm.current}
          <Button size="sm" variant="secondary" onclick={() => llm.start(llm.current!.id)}>Try again</Button>
        {/if}
        <Button size="sm" variant="ghost" href="/models">Open Models</Button>
      </span>
    </AlertDescription>
  </Alert>
{/if}
