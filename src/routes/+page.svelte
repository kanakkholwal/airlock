<script lang="ts">
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Card, CardContent, CardHeader, CardTitle } from "#lib/components/ui/card/index.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import {
    Composer,
    ComposerInput,
    ComposerSubmit,
    ComposerToolbar,
  } from "#lib/components/agents/composer/index.js";

  let mode = $state("chat");
  let draft = $state("");
  let queued = $state<string[]>([]);

  const placeholders: Record<string, string> = {
    chat: "Ask anything",
    dictation: "Dictation runs from the hotkey",
    image: "Describe an image",
  };

  function send(text: string) {
    // Messages wait for the model instead of disabling the input (plan §14).
    queued = [...queued, text];
    draft = "";
  }
</script>

<main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-16">
  <header class="text-center">
    <h1 class="text-3xl font-semibold tracking-tight text-balance">Airlock</h1>
    <p class="mt-2 text-muted-foreground text-pretty">
      Local models, with you deciding what crosses the boundary.
    </p>
  </header>

  <section class="flex flex-col gap-3 rounded-2xl bg-card p-2">
    <Tabs bind:value={mode}>
      <TabsList>
        <TabsTrigger value="chat">Chat</TabsTrigger>
        <TabsTrigger value="dictation">Dictation</TabsTrigger>
        <TabsTrigger value="image">Image</TabsTrigger>
      </TabsList>
    </Tabs>
    <Composer bind:value={draft} onSubmit={send} variant="filled">
      <ComposerInput placeholder={placeholders[mode]} />
      <ComposerToolbar>
        <ComposerSubmit />
      </ComposerToolbar>
    </Composer>
    {#if queued.length > 0}
      <p class="px-3 pb-1 text-sm text-muted-foreground">
        {queued.length} waiting for a model
      </p>
    {/if}
  </section>

  <section class="grid gap-3 sm:grid-cols-2">
    <Card variant="secondary">
      <CardHeader><CardTitle>Engine</CardTitle></CardHeader>
      <CardContent class="flex items-center gap-2">
        <Badge variant="secondary">Not probed</Badge>
        <span class="text-sm text-muted-foreground">Vulkan, CUDA or CPU is chosen at startup.</span>
      </CardContent>
    </Card>
    <Card variant="secondary">
      <CardHeader><CardTitle>Model</CardTitle></CardHeader>
      <CardContent class="flex items-center gap-2">
        <Badge variant="secondary">Not downloaded</Badge>
        <span class="text-sm text-muted-foreground">LFM3-1.7B is the default.</span>
      </CardContent>
    </Card>
  </section>
</main>
