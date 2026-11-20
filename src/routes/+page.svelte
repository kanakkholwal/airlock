<script lang="ts">
  import { chat } from "#lib/chat.svelte.js";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import {
    Composer,
    ComposerInput,
    ComposerSubmit,
    ComposerToolbar,
  } from "#lib/components/agents/composer/index.js";
  import {
    Conversation,
    ConversationContent,
    ConversationEmpty,
    ConversationScrollButton,
  } from "#lib/components/agents/conversation/index.js";
  import {
    Message,
    MessageBubble,
    MessageContent,
    MessageTyping,
  } from "#lib/components/agents/message/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { EmptyDescription, EmptyHeader, EmptyTitle } from "#lib/components/ui/empty/index.js";

  let draft = $state("");

  const engineLabel = {
    unavailable: "No engine",
    loading: "Loading model",
    ready: "Ready",
    error: "Engine error",
  } as const;
  const engineVariant = {
    unavailable: "secondary",
    loading: "info",
    ready: "success",
    error: "destructive",
  } as const;

  function textOf(parts: (typeof chat.messages)[number]["parts"]) {
    return parts.flatMap((p) => (p.type === "text" ? [p.text] : [])).join("");
  }
</script>

<main class="mx-auto flex h-full w-full max-w-3xl flex-col gap-4 px-6 pt-6 pb-6">
  {#if chat.messages.length === 0}
    <PageHeader title="Airlock" description="Local models, with you deciding what crosses the boundary." />
  {/if}

  <div class="min-h-0 flex-1">
    <Conversation class="h-full">
      <ConversationContent>
        {#if chat.messages.length === 0}
          <ConversationEmpty>
            <EmptyHeader>
              <EmptyTitle>Start a conversation</EmptyTitle>
              <EmptyDescription>Messages wait for the model if it is still loading.</EmptyDescription>
            </EmptyHeader>
          </ConversationEmpty>
        {/if}
        {#each chat.messages as m (m.id)}
          <Message align={m.role === "user" ? "end" : "start"} aria-label={m.role}>
            <MessageContent>
              <MessageBubble variant={m.role === "user" ? "primary" : m.status === "error" ? "destructive" : "ghost"}>
                {#if m.status === "streaming" && textOf(m.parts) === ""}
                  <MessageTyping />
                {:else}
                  {textOf(m.parts)}
                {/if}
              </MessageBubble>
            </MessageContent>
          </Message>
        {/each}
      </ConversationContent>
      <ConversationScrollButton />
    </Conversation>
  </div>

  <section class="flex flex-col gap-2 rounded-2xl bg-card p-2">
    <Composer
      bind:value={draft}
      variant="filled"
      loading={chat.busy}
      onStop={() => chat.stop()}
      onSubmit={(text) => {
        chat.send(text);
        draft = "";
      }}
    >
      <ComposerInput placeholder="Ask anything" />
      <ComposerToolbar>
        <ComposerSubmit />
      </ComposerToolbar>
    </Composer>
    <div class="flex items-center gap-2 px-3 pb-1 text-sm text-muted-foreground">
      <Badge size="sm" variant={engineVariant[chat.engine]}>{engineLabel[chat.engine]}</Badge>
      {#if chat.queuedCount > 0}
        <span>{chat.queuedCount} waiting for the model</span>
      {/if}
    </div>
  </section>
</main>
