<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { chat, imagesOf, reasoningOf, textOf } from "#lib/chat.svelte.js";
  import ChatComposer from "#lib/components/app/chat-composer.svelte";
  import EngineBanner from "#lib/components/app/engine-banner.svelte";
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import {
    Attachment,
    AttachmentPreview,
    Attachments,
  } from "#lib/components/agents/attachment/index.js";
  import {
    Conversation,
    ConversationContent,
    ConversationScrollButton,
  } from "#lib/components/agents/conversation/index.js";
  import {
    Message,
    MessageBubble,
    MessageContent,
    MessageTyping,
  } from "#lib/components/agents/message/index.js";
  import {
    MessageActions,
    MessageActionsBar,
    MessageActionsCopy,
    MessageActionsRegenerate,
  } from "#lib/components/agents/message-actions/index.js";
  import { Reasoning } from "#lib/components/agents/reasoning/index.js";
  import {
    StreamingText,
    StreamingTextContent,
    StreamingTextFooter,
  } from "#lib/components/agents/streaming-text/index.js";

  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? "image";
</script>

{#if chat.messages.length === 0}
  <main class="mx-auto flex h-full w-full max-w-3xl flex-col justify-center gap-8 px-6 pb-24">
    <PageHeader title="What can I help with?" description="Everything runs on this machine." />
    {#if chat.opening}
      <div class="flex flex-col gap-3" role="status" aria-label="Opening chat">
        <Skeleton class="h-5 w-2/3 self-end" />
        <Skeleton class="h-16 w-full" />
        <Skeleton class="h-5 w-1/2 self-end" />
      </div>
    {/if}
    <EngineBanner />
    <ChatComposer />
  </main>
{:else}
  <main class="mx-auto flex h-full w-full max-w-3xl flex-col gap-4 px-6 pb-6">
    <div class="min-h-0 flex-1">
      <Conversation class="h-full">
        <ConversationContent>
          {#each chat.messages as m, i (m.id)}
            {@const text = textOf(m)}
            {@const thoughts = reasoningOf(m)}
            {@const images = imagesOf(m)}
            {@const streaming = m.status === "streaming"}
            {#if m.role === "user"}
              <Message align="end" aria-label="You">
                <MessageContent>
                  {#if images.length > 0}
                    <Attachments variant="wrap">
                      {#each images as path (path)}
                        <Attachment variant="tile" name={fileName(path)}>
                          <AttachmentPreview src={convertFileSrc(path)} />
                        </Attachment>
                      {/each}
                    </Attachments>
                  {/if}
                  {#if text}
                    <MessageBubble variant="primary">{text}</MessageBubble>
                  {/if}
                </MessageContent>
              </Message>
            {:else}
              <Message align="start" aria-label="Assistant">
                <MessageContent class="w-full">
                  {#if thoughts}
                    <Reasoning thinking={streaming && !text} duration={m.thinkingSeconds ?? 0}>
                      <p class="text-sm whitespace-pre-wrap text-muted-foreground">{thoughts}</p>
                    </Reasoning>
                  {/if}
                  <MessageBubble variant={m.status === "error" ? "destructive" : "ghost"}>
                    {#if m.status === "error"}
                      {m.error ?? "The reply failed."}
                    {:else if streaming && !text && !thoughts}
                      <MessageTyping />
                    {:else if text}
                      <StreamingText layout="inline">
                        <StreamingTextContent markdown content={text} {streaming} />
                        {#if m.status === "done"}
                          <StreamingTextFooter>
                            <MessageActions>
                              <MessageActionsBar>
                                <MessageActionsCopy {text} />
                                {#if i === chat.messages.length - 1}
                                  <MessageActionsRegenerate onclick={() => chat.regenerate()} />
                                {/if}
                              </MessageActionsBar>
                            </MessageActions>
                            {#if m.tokensPerSecond}
                              <span class="text-xs text-muted-foreground tabular-nums">
                                {m.tokensPerSecond.toFixed(0)} tok/s
                              </span>
                            {/if}
                          </StreamingTextFooter>
                        {/if}
                      </StreamingText>
                    {/if}
                  </MessageBubble>
                </MessageContent>
              </Message>
            {/if}
          {/each}
        </ConversationContent>
        <ConversationScrollButton />
      </Conversation>
    </div>
    <EngineBanner />
    <ChatComposer />
  </main>
{/if}
