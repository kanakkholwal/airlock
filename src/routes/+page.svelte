<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { chat, hasTools, imagesOf, reasoningOf, referencesOf, resultOf, textOf } from "#lib/chat.svelte.js";
  import { speaker } from "#lib/speak.svelte.js";
  import ChatComposer from "#lib/components/app/chat-composer.svelte";
  import ChatToolCall from "#lib/components/app/chat-tool-call.svelte";
  import KindIcon from "#lib/components/app/kind-icon.svelte";
  import EngineBanner from "#lib/components/app/engine-banner.svelte";
  import FirstRun from "#lib/components/app/first-run.svelte";
  import { llm } from "#lib/llm.svelte.js";
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
    MessageActionsSpeak,
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
    {#if llm.needsSetup || llm.setup}<FirstRun />{:else}<EngineBanner />{/if}
    <ChatComposer />
  </main>
{:else}
  <main class="mx-auto flex h-full w-full max-w-5xl flex-col gap-4 px-6 pb-6">
    <!-- Decision (product): the transcript is wider than the box you type in. Bubbles are sized
         against this container, not the window, so they stay readable at any width. -->
    <div class="@container min-h-0 flex-1">
      <Conversation class="h-full">
        <ConversationContent>
          {#each chat.messages as m, i (m.id)}
            {@const text = textOf(m)}
            {@const thoughts = reasoningOf(m)}
            {@const images = imagesOf(m)}
            {@const streaming = m.status === "streaming"}
            {#if m.role === "user"}
              <Message align="end" aria-label="You" animated={m.fresh === true}>
                <MessageContent class="max-w-[80cqw]">
                  {#if images.length > 0}
                    <Attachments variant="wrap">
                      {#each images as path (path)}
                        <Attachment variant="tile" name={fileName(path)}>
                          <AttachmentPreview src={convertFileSrc(path)} />
                        </Attachment>
                      {/each}
                    </Attachments>
                  {/if}
                  {#each [referencesOf(m)] as refs (m.id)}
                    {#if refs.length > 0}
                      <ul class="flex flex-wrap justify-end gap-1.5">
                        {#each refs as r (r.id)}
                          <li class="flex items-center gap-1.5 rounded-full bg-muted py-1 px-2.5 text-sm text-muted-foreground">
                            <KindIcon kind={r.kind} size={14} /><span class="max-w-48 truncate">{r.title}</span>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  {/each}
                  {#if text}
                    <MessageBubble variant="primary">{text}</MessageBubble>
                  {/if}
                </MessageContent>
              </Message>
            {:else}
              <Message align="start" aria-label="Assistant" animated={m.fresh === true}>
                <MessageContent class="w-full max-w-[80cqw]">
                  {#if hasTools(m)}
                    <!-- A reply that used tools is shown step by step, in the order it happened. -->
                    <div class="flex flex-col gap-2">
                      {#each m.parts as p, pi (pi)}
                        {@const last = pi === m.parts.length - 1}
                        {#if p.type === "reasoning"}
                          <Reasoning thinking={streaming && last} duration={m.thinkingSeconds ?? 0}>
                            <p class="text-sm whitespace-pre-wrap text-muted-foreground">{p.text}</p>
                          </Reasoning>
                        {:else if p.type === "text"}
                          <MessageBubble variant="ghost">
                            <StreamingText layout="inline">
                              <StreamingTextContent markdown content={p.text} streaming={streaming && last} />
                            </StreamingText>
                          </MessageBubble>
                        {:else if p.type === "tool_call"}
                          <ChatToolCall call={p} result={resultOf(m, p.call_id)} approval={chat.approvals[p.call_id]} />
                        {/if}
                      {/each}
                      {#if streaming && m.parts.at(-1)?.type === "tool_result"}<MessageTyping />{/if}
                      {#if m.status === "error"}
                        <MessageBubble variant="destructive">{m.error ?? "The reply failed."}</MessageBubble>
                      {:else if m.status === "done" && text}
                        <MessageActions>
                          <MessageActionsBar>
                            <MessageActionsCopy {text} />
                            {#if i === chat.messages.length - 1}
                              <MessageActionsRegenerate onclick={() => chat.regenerate()} />
                            {/if}
                          </MessageActionsBar>
                        </MessageActions>
                      {/if}
                    </div>
                  {:else}
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
                                {#if speaker.available}
                                  <MessageActionsSpeak
                                    pressed={speaker.speakingId === String(m.id)}
                                    onPressedChange={(on) => (on ? speaker.speak(text, String(m.id)) : speaker.stop())}
                                  />
                                {/if}
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
                  {/if}
                </MessageContent>
              </Message>
            {/if}
          {/each}
        </ConversationContent>
        <ConversationScrollButton />
      </Conversation>
    </div>
    <div class="mx-auto flex w-full max-w-3xl flex-col gap-4">
      <EngineBanner />
      <ChatComposer />
    </div>
  </main>
{/if}
