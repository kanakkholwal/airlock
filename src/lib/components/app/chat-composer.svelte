<script lang="ts">
  import { chat } from "#lib/chat.svelte.js";
  import { draft } from "#lib/draft.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import ChatStats from "#lib/components/app/chat-stats.svelte";
  import {
    Attachment,
    AttachmentContent,
    AttachmentMeta,
    AttachmentName,
    AttachmentPreview,
    AttachmentRemove,
    Attachments,
  } from "#lib/components/agents/attachment/index.js";
  import {
    Composer,
    ComposerActions,
    ComposerInput,
    ComposerModelSelect,
    ComposerSubmit,
    ComposerToolbar,
  } from "#lib/components/agents/composer/index.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";

  // Decision (product): the composer carries what chat needs. A model picker that handles
  // loading and memory itself, image attach only when the model can see, and the context
  // meter. Image-size pills belong to an image composer, not here.
  const hasEngine = $derived(llm.engines.some((e) => e.installed));
  const models = $derived(llm.downloaded.map((m) => ({ value: m.id, label: m.name })));

  const status = $derived.by(() => {
    if (chat.engine === "ready") return { label: "Ready", variant: "success" as const, hint: "" };
    if (chat.engine === "loading") return { label: "Loading model", variant: "info" as const, hint: "" };
    if (chat.engine === "error") return { label: "Engine error", variant: "destructive" as const, hint: "Open Models to retry." };
    return {
      label: "No model loaded",
      variant: "secondary" as const,
      hint: models.length > 0 ? "Pick a model below." : hasEngine ? "Download a model in Models." : "Install an engine in Models.",
    };
  });

  // Only real messages, so keys are never null and identical messages show once.
  const problems = $derived(
    [...new Set([draft.error, llm.error, chat.saveError ? `Not saved: ${chat.saveError}` : null])].filter(
      (p): p is string => Boolean(p),
    ),
  );

  const attachActions = $derived([
    {
      value: "image",
      label: "Attach image",
      description: llm.canSeeImages
        ? "PNG, JPEG, GIF or WebP"
        : "Load a vision model such as LFM3 VL to attach images",
      disabled: !llm.canSeeImages,
    },
  ]);

  const kb = (bytes: number) => (bytes >= 1_048_576 ? `${(bytes / 1_048_576).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`);

  function submit(text: string) {
    chat.send(
      text,
      draft.images.map((i) => ({ path: i.path, name: i.name })),
    );
    draft.clear();
  }
</script>

<section class="flex flex-col gap-2 rounded-2xl bg-card p-2">
  <Composer
    bind:value={draft.text}
    variant="filled"
    loading={chat.busy}
    onStop={() => chat.stop()}
    onSubmit={submit}
  >
    {#if draft.images.length > 0 || draft.pending.length > 0}
      <Attachments variant="scroll">
        {#each draft.images as img (img.path)}
          <Attachment name={img.name}>
            <AttachmentPreview src={img.src} />
            <AttachmentContent>
              <AttachmentName />
              <AttachmentMeta>{kb(img.sizeBytes)}</AttachmentMeta>
            </AttachmentContent>
            <AttachmentRemove onclick={() => draft.remove(img.path)} />
          </Attachment>
        {/each}
        {#each draft.pending as name (name)}
          <Attachment {name} status="uploading">
            <AttachmentPreview />
            <AttachmentContent>
              <AttachmentName />
              <AttachmentMeta>Adding</AttachmentMeta>
            </AttachmentContent>
          </Attachment>
        {/each}
      </Attachments>
    {/if}
    <ComposerInput placeholder="Ask anything" minRows={3} />
    <ComposerToolbar>
      <ComposerActions actions={attachActions} onAction={() => draft.pickImages()} label="Add to message" />
      <ChatStats />
      <div class="ms-auto flex items-center gap-1">
        {#if models.length > 0}
          <ComposerModelSelect
            {models}
            bind:model={() => llm.current?.id ?? "", (id) => void llm.switchTo(id)}
          />
        {:else}
          <Button size="sm" variant="secondary" href="/models">Get a model</Button>
        {/if}
        <ComposerSubmit />
      </div>
    </ComposerToolbar>
  </Composer>

  <div class="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 pb-1 text-sm text-muted-foreground">
    <Badge size="sm" variant={status.variant}>{status.label}</Badge>
    {#if status.hint}
      <span>{status.hint}</span>
    {/if}
    {#if chat.queuedCount > 0}
      <span>{chat.queuedCount} waiting for the model</span>
    {/if}
  </div>

  {#each problems as problem (problem)}
    <Alert variant="destructive">
      <AlertDescription>{problem}</AlertDescription>
    </Alert>
  {/each}
</section>
