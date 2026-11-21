<script lang="ts">
  import { commands, type Artifact } from "../../../bindings";
  import { chat } from "#lib/chat.svelte.js";
  import KindIcon from "#lib/components/app/kind-icon.svelte";
  import MentionList from "#lib/components/app/mention-list.svelte";
  import { draft } from "#lib/draft.svelte.js";
  import { mentionQuery, withoutMention } from "#lib/mention.js";
  import { llm } from "#lib/llm.svelte.js";
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
      hint: models.length > 0 ? "It loads when you send a message." : hasEngine ? "Download a model in Models." : "Install an engine in Models.",
    };
  });

  // Only real messages, so keys are never null and identical messages show once.
  const problems = $derived(
    [...new Set([draft.error, llm.error, chat.saveError ? `Not saved: ${chat.saveError}` : null])].filter(
      (p): p is string => Boolean(p),
    ),
  );

  // --- @ mentions ---------------------------------------------------------------------------
  // Decision (product): typing @ at the end of the text, or choosing "Mention from the library",
  // opens a short list of your recordings, files, notes, summaries and images. A pick becomes a
  // chip; only its title and a short preview travel with the message.
  const openQuery = $derived(mentionQuery(draft.text));
  let suggestions = $state<Artifact[]>([]);
  let activeMention = $state(0);
  let indexed = false;
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const q = openQuery;
    clearTimeout(suggestTimer);
    if (q === null) return;
    suggestTimer = setTimeout(async () => {
      if (!indexed) {
        indexed = true;
        await commands.libraryRefresh();
      }
      const res = await commands.libraryList(null, q || null);
      if (res.status === "ok") {
        const taken = new Set(draft.mentions.map((m) => m.id));
        suggestions = res.data.filter((a) => !taken.has(a.id)).slice(0, 6);
        activeMention = 0;
      }
    }, 120);
    return () => clearTimeout(suggestTimer);
  });

  function pickMention(item: Artifact) {
    draft.addMention(item);
    draft.text = withoutMention(draft.text);
    suggestions = [];
  }

  // The composer would send on Enter, so the list gets the keys first while it is open.
  function onKeyCapture(e: KeyboardEvent) {
    if (openQuery === null) return;
    const last = suggestions.length - 1;
    if (e.key === "Escape") {
      draft.text = withoutMention(draft.text);
    } else if (suggestions.length === 0) {
      return;
    } else if (e.key === "ArrowDown") {
      activeMention = activeMention >= last ? 0 : activeMention + 1;
    } else if (e.key === "ArrowUp") {
      activeMention = activeMention <= 0 ? last : activeMention - 1;
    } else if (e.key === "Enter" || e.key === "Tab") {
      pickMention(suggestions[activeMention]);
    } else {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  const attachActions = $derived([
    {
      value: "mention",
      label: "Mention from the library",
      description: "Point at a recording, file, note or image. Only a short preview is shared.",
    },
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
      draft.mentions.map((m) => ({ id: m.id, title: m.title, kind: m.kind, preview: m.preview })),
    );
    draft.clear();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<section class="relative flex flex-col gap-2 rounded-2xl bg-card p-2" onkeydowncapture={onKeyCapture}>
  {#if openQuery !== null}
    <MentionList items={suggestions} active={activeMention} onpick={pickMention} />
  {/if}
  {#if draft.mentions.length > 0}
    <ul class="flex flex-wrap gap-1.5 px-1 pt-1" aria-label="Mentioned from the library">
      {#each draft.mentions as m (m.id)}
        <li class="flex items-center gap-1.5 rounded-full bg-muted py-1 ps-2.5 pe-1 text-sm motion-safe:animate-in motion-safe:fade-in motion-safe:zoom-in-95 motion-safe:duration-150">
          <KindIcon kind={m.kind} size={14} class="text-muted-foreground" />
          <span class="max-w-48 truncate">{m.title}</span>
          <button
            type="button"
            class="flex size-5 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            aria-label={`Remove ${m.title}`}
            onclick={() => draft.removeMention(m.id)}
          >
            <svg viewBox="0 0 16 16" class="size-2.5" aria-hidden="true"><path d="M4 4 L12 12 M12 4 L4 12" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" fill="none" /></svg>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
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
      <ComposerActions
        actions={attachActions}
        onAction={(value) => (value === "mention" ? (draft.text = `${draft.text}${draft.text && !draft.text.endsWith(" ") ? " " : ""}@`) : draft.pickImages())}
        label="Add to message"
      />
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
    {#if chat.engine === "unavailable" && models.length > 0 && hasEngine}
      <Button size="sm" variant="ghost" onclick={() => llm.ensureLoaded()}>Load now</Button>
    {:else if chat.engine === "ready"}
      <Button size="sm" variant="ghost" onclick={() => llm.stop()}>Unload to free memory</Button>
    {/if}
  </div>

  {#each problems as problem (problem)}
    <Alert variant="destructive">
      <AlertDescription>{problem}</AlertDescription>
    </Alert>
  {/each}
</section>
