<script lang="ts">
  import { AddCircleIcon, DangerTriangleIcon } from "@solar-icons/svelte/linear";
  import ModelPicker, { type PickerModel } from "#lib/components/app/model-picker.svelte";
  import { providerOf } from "#lib/model-provider.js";
  import { formatBytes } from "#lib/format.js";
  import { commands, type Artifact, type FileHit } from "../../../bindings";
  import { chat } from "#lib/chat.svelte.js";
  import KindIcon from "#lib/components/app/kind-icon.svelte";
  import MentionList, { type MentionRow } from "#lib/components/app/mention-list.svelte";
  import { draft } from "#lib/draft.svelte.js";
  import {
    CATEGORIES,
    categoriesMatching,
    categoryFor,
    mentionQuery,
    withBareMention,
    withoutMention,
    type CategoryId,
  } from "#lib/mention.js";
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
  const models = $derived(llm.downloaded);
  const pickerModels = $derived<PickerModel[]>(
    models.map((m) => ({ id: m.id, name: m.name, provider: providerOf(m), description: m.description, meta: formatBytes(m.size_bytes), speed: m.speed ?? undefined, intelligence: m.intelligence ?? undefined })),
  );

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
  // Decision (product): `@` shows the groups (recordings, notes, files, and so on), a group opens
  // to the things in it, and a word that is not a group searches everything. A pick becomes a
  // chip; only its title and a short preview travel with the message.
  const openQuery = $derived(mentionQuery(draft.text));
  let picked = $state<CategoryId | null>(null);
  let rows = $state<MentionRow[]>([]);
  let activeMention = $state(0);
  let indexed = false;
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;

  // The group in play: one chosen from the list, or one named by typing, such as @notes.
  const group = $derived(picked ?? (openQuery ? (categoryFor(openQuery)?.id ?? null) : null));
  // After a group is chosen, further typing filters inside it; otherwise the word is the search.
  const term = $derived(openQuery === null ? "" : picked ? openQuery : categoryFor(openQuery) ? "" : openQuery);
  const heading = $derived(CATEGORIES.find((c) => c.id === group)?.label);
  const emptyText = $derived(
    group === "file"
      ? "No files found. Share a folder in Settings, Assistant."
      : group
        ? `No ${heading?.toLowerCase()} yet.`
        : "Nothing matches.",
  );

  const asItem = (item: Artifact): MentionRow => ({ type: "item", item });
  const fileItem = (hit: FileHit): Artifact => ({
    id: `file:${hit.path}`,
    kind: "file",
    title: hit.name,
    preview: hit.path,
    created_ms: 0,
    privacy: "local_only",
    source: hit.folder,
    path: hit.path,
  });

  async function itemsFor(g: CategoryId | null, t: string): Promise<Artifact[]> {
    if (g === "file") return (await commands.agentSearchFiles(t)).map(fileItem);
    const res = await commands.libraryList(g, t || null);
    const found = res.status === "ok" ? res.data : [];
    // A search across everything also looks in shared folders.
    return g === null && t ? [...found, ...(await commands.agentSearchFiles(t)).slice(0, 3).map(fileItem)] : found;
  }

  $effect(() => {
    const q = openQuery;
    const g = group;
    const t = term;
    clearTimeout(suggestTimer);
    if (q === null) {
      picked = null;
      rows = [];
      return;
    }
    suggestTimer = setTimeout(async () => {
      const groups: MentionRow[] =
        g === null ? categoriesMatching(t).map((c) => ({ type: "group", id: c.id, label: c.label })) : [];
      // Nothing typed yet: just the groups.
      if (g === null && t === "") {
        rows = groups;
        activeMention = 0;
        return;
      }
      if (!indexed) {
        indexed = true;
        await commands.libraryRefresh();
      }
      const taken = new Set(draft.mentions.map((m) => m.id));
      const items = (await itemsFor(g, t)).filter((a) => !taken.has(a.id)).slice(0, 6);
      rows = [...groups, ...items.map(asItem)];
      activeMention = 0;
    }, 100);
    return () => clearTimeout(suggestTimer);
  });

  function pickMention(row: MentionRow) {
    if (row.type === "group") {
      picked = row.id;
      draft.text = withBareMention(draft.text);
      return;
    }
    draft.addMention(row.item);
    draft.text = withoutMention(draft.text);
    picked = null;
    rows = [];
  }

  // The composer would send on Enter, so the list gets the keys first while it is open.
  function onKeyCapture(e: KeyboardEvent) {
    if (openQuery === null) return;
    const last = rows.length - 1;
    if (e.key === "Escape") {
      draft.text = withoutMention(draft.text);
      picked = null;
    } else if (e.key === "Backspace" && openQuery === "" && picked) {
      // Back out of a group to the groups, keeping the @.
      picked = null;
    } else if (rows.length === 0) {
      return;
    } else if (e.key === "ArrowDown") {
      activeMention = activeMention >= last ? 0 : activeMention + 1;
    } else if (e.key === "ArrowUp") {
      activeMention = activeMention <= 0 ? last : activeMention - 1;
    } else if (e.key === "Enter" || e.key === "Tab") {
      pickMention(rows[activeMention]);
    } else {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
  }

  const attachActions = $derived([
    {
      value: "mention",
      label: "Mention",
      description: "A recording, note, image or file",
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

  // A new hint every few seconds while the box is empty and nothing is running, so the tips
  // teach @ and attachments without taking space. Still while typing or reduced motion.
  const PLACEHOLDERS = ["Ask anything", "Type @ to mention a recording or file", "Ask about a note or an image"];
  let hintIndex = $state(0);
  const placeholder = $derived(draft.text || chat.busy ? PLACEHOLDERS[0] : PLACEHOLDERS[hintIndex % PLACEHOLDERS.length]);
  $effect(() => {
    if (draft.text || chat.busy || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const id = setInterval(() => (hintIndex += 1), 4000);
    return () => clearInterval(id);
  });

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
    <MentionList {rows} active={activeMention} onpick={pickMention} {heading} empty={emptyText} />
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
    <ComposerInput placeholder={placeholder} minRows={2} />
    <ComposerToolbar>
      <ComposerActions
        actions={attachActions}
        onAction={(value) => (value === "mention" ? (draft.text = `${draft.text}${draft.text && !draft.text.endsWith(" ") ? " " : ""}@`) : draft.pickImages())}
        label="Add to message"
      />
      <div class="ms-auto flex items-center gap-1">
        {#if models.length > 0}
          <ModelPicker
            models={pickerModels}
            value={llm.current?.id}
            onchange={(id) => void llm.switchTo(id)}
          >
            {#snippet footer()}
              <a
                href="/models"
                class="flex h-9 items-center gap-2.5 rounded-xl px-2.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              >
                <AddCircleIcon size={16} />
                Get more models
              </a>
            {/snippet}
          </ModelPicker>
        {:else}
          <Button size="sm" variant="secondary" href="/models">Get a model</Button>
        {/if}
        <ComposerSubmit />
      </div>
    </ComposerToolbar>
  </Composer>

  {#if chat.engine !== "ready" || chat.queuedCount > 0}
    <div class="flex flex-wrap items-center gap-x-2 gap-y-1 px-3 pb-1 text-sm text-muted-foreground motion-safe:animate-in motion-safe:fade-in motion-safe:duration-200">
      {#if chat.engine !== "ready"}
        <Badge size="sm" variant={status.variant}>{status.label}</Badge>
        {#if status.hint}<span>{status.hint}</span>{/if}
      {/if}
      {#if chat.queuedCount > 0}
        <span class="tabular-nums">{chat.queuedCount} waiting</span>
      {/if}
      {#if chat.engine === "unavailable" && models.length > 0 && hasEngine}
        <Button size="sm" variant="ghost" onclick={() => llm.ensureLoaded()}>Load now</Button>
      {/if}
    </div>
  {/if}

  {#each problems as problem (problem)}
    <Alert variant="destructive"><DangerTriangleIcon size={18} />
      <AlertDescription>{problem}</AlertDescription>
    </Alert>
  {/each}
</section>
