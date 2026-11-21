<script lang="ts">
  import { MenuDotsIcon } from "@solar-icons/svelte/linear";
  import LoadedModels from "#lib/components/app/loaded-models.svelte";
  import ChatStats from "#lib/components/app/chat-stats.svelte";
  import { HoverMarquee } from "#lib/components/text/hover-marquee/index.js";
  import { chat } from "#lib/chat.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import DeleteChatDialog from "#lib/components/app/delete-chat-dialog.svelte";
  import { Button } from "#lib/components/ui/button/index.js";
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
  } from "#lib/components/ui/dropdown-menu/index.js";
  import { Input } from "#lib/components/ui/input/index.js";

  // Decision (product): the header says where you are and links what the chat runs on. The
  // title updates when the auto title arrives, which is the feedback that it worked.
  const title = $derived(chat.currentTitle ?? "New chat");
  const canManage = $derived(chat.conversationId !== null);


  let renaming = $state(false);
  let draftTitle = $state("");
  let confirmDelete = $state(false);

  function startRename() {
    draftTitle = chat.currentTitle ?? "";
    renaming = true;
  }

  async function commitRename() {
    renaming = false;
    if (chat.conversationId !== null) await chat.rename(chat.conversationId, draftTitle);
  }

</script>

<header class="flex h-12 shrink-0 items-center justify-between gap-3 px-4">
  <div class="flex min-w-0 items-center gap-2">
    {#if renaming}
      <Input
        class="h-8 w-72 max-w-full"
        aria-label="Chat title"
        bind:value={draftTitle}
        onkeydown={(e) => {
          if (e.key === "Enter") void commitRename();
          if (e.key === "Escape") renaming = false;
        }}
        onblur={() => void commitRename()}
        autofocus
      />
    {:else}
      <h1 class="min-w-0 text-sm font-medium"><HoverMarquee text={title} /></h1>
    {/if}

    {#if canManage && !renaming}
      <DropdownMenu>
        <DropdownMenuTrigger>
          {#snippet child({ props })}
            <Button {...props} variant="ghost" size="sm" aria-label="Chat actions">
              <MenuDotsIcon size={18} />
            </Button>
          {/snippet}
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          <DropdownMenuItem onclick={startRename}>Rename</DropdownMenuItem>
          <DropdownMenuItem onclick={() => (confirmDelete = true)}>Delete chat</DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
    {/if}

  </div>

  <div class="flex shrink-0 items-center gap-1">
    <ChatStats />
    <LoadedModels />
  </div>
</header>

<DeleteChatDialog bind:open={confirmDelete} id={chat.conversationId} title={chat.currentTitle ?? "This chat"} />