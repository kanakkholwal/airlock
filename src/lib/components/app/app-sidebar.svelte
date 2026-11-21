<script lang="ts">
  import HoverMarquee from "#lib/components/app/hover-marquee.svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { exit } from "@tauri-apps/plugin-process";
  import {
    ChatLineIcon,
    FolderWithFilesIcon,
    GalleryIcon,
    VolumeLoudIcon,
    CpuIcon,
    FolderOpenIcon,
    GlobalIcon,
    MenuDotsIcon,
    MicrophoneIcon,
    PenNewSquareIcon,
    PowerIcon,
    SettingsIcon,
    SidebarMinimalisticIcon,
  } from "@solar-icons/svelte/linear";
  import { commands } from "../../../bindings";
  import { APP_URL } from "#lib/app-info.js";
  import { chat } from "#lib/chat.svelte.js";
  import { groupByRecency } from "#lib/chat-groups.js";
  import { inTauri } from "#lib/env.js";
  import { llm } from "#lib/llm.svelte.js";
  import DeleteChatDialog from "#lib/components/app/delete-chat-dialog.svelte";
  import Logo from "#lib/components/app/logo.svelte";
  import { Button } from "#lib/components/ui/button/index.js";
  import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
  } from "#lib/components/ui/dropdown-menu/index.js";
  import { Input } from "#lib/components/ui/input/index.js";
  import { ScrollArea } from "#lib/components/ui/scroll-area/index.js";
  import { Skeleton } from "#lib/components/ui/skeleton/index.js";

  // Decision (product): the sidebar answers four questions: where can I go, what did I talk
  // about, can I chat right now, and how do I leave. Chats group themselves by recency, a
  // search box appears only when the list is long enough to need one, and presses give
  // instant feedback (no scale or glide) because this is the most-clicked surface in the app.
  const SEARCH_FROM = 8;
  const STORAGE_KEY = "airlock-sidebar-collapsed";

  let collapsed = $state(typeof localStorage !== "undefined" && localStorage.getItem(STORAGE_KEY) === "1");
  let query = $state("");
  let renamingId = $state<number | null>(null);
  let renameText = $state("");
  let deleting = $state<{ id: number; title: string } | null>(null);
  let deleteOpen = $state(false);

  function toggle() {
    collapsed = !collapsed;
    localStorage.setItem(STORAGE_KEY, collapsed ? "1" : "0");
  }

  const nav = [
    { href: "/dictation", label: "Dictation", icon: dictationIcon },
    { href: "/voice", label: "Voice", icon: voiceIcon },
    { href: "/images", label: "Images", icon: imagesIcon },
    { href: "/library", label: "Library", icon: libraryIcon },
    { href: "/models", label: "Models", icon: modelsIcon },
    { href: "/settings", label: "Settings", icon: settingsIcon },
  ];

  const filtered = $derived(
    query.trim()
      ? chat.conversations.filter((c) => c.title.toLowerCase().includes(query.trim().toLowerCase()))
      : chat.conversations,
  );
  const groups = $derived(groupByRecency(filtered));
  const onChat = $derived(page.url.pathname === "/");

  const status = $derived.by(() => {
    const e = llm.engine;
    const name = llm.current?.name ?? "Model";
    if (e.state === "ready") return { label: `${name} ready`, dot: "bg-success" };
    if (e.state === "loading") return { label: `Loading ${name}`, dot: "bg-info animate-pulse" };
    if (e.state === "failed") return { label: "Engine problem", dot: "bg-destructive" };
    return { label: llm.downloaded.length > 0 ? "No model loaded" : "Set up a model", dot: "bg-muted-foreground/50" };
  });

  function newChat() {
    chat.newChat();
    void goto("/");
  }

  function openChat(id: number) {
    void chat.open(id);
    void goto("/");
  }

  function startRename(id: number, title: string) {
    renamingId = id;
    renameText = title;
  }

  async function commitRename() {
    const id = renamingId;
    renamingId = null;
    if (id !== null) await chat.rename(id, renameText);
  }

  function askDelete(id: number, title: string) {
    deleting = { id, title };
    deleteOpen = true;
  }

  // The logo menu is gone; what it held now lives next to the status, where leaving the app
  // belongs. Closing the window does not quit (the app lives in the tray), so Quit is here.
  const row =
    "flex h-8 w-full items-center gap-2.5 rounded-lg px-2 text-sm text-muted-foreground outline-none transition-colors duration-100 hover:bg-foreground/[0.06] hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[active=true]:bg-foreground/[0.09] data-[active=true]:font-medium data-[active=true]:text-foreground";
</script>

<svelte:window
  onkeydown={(e) => {
    const typing = e.target instanceof HTMLElement && e.target.closest("input, textarea, [contenteditable]");
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "b" && !typing) {
      e.preventDefault();
      toggle();
    }
  }}
/>

{#snippet dictationIcon()}<MicrophoneIcon size={18} />{/snippet}
{#snippet voiceIcon()}<VolumeLoudIcon size={18} />{/snippet}
{#snippet libraryIcon()}<FolderWithFilesIcon size={18} />{/snippet}
{#snippet imagesIcon()}<GalleryIcon size={18} />{/snippet}
{#snippet modelsIcon()}<CpuIcon size={18} />{/snippet}
{#snippet settingsIcon()}<SettingsIcon size={18} />{/snippet}

<aside
  aria-label="Airlock navigation"
  class="relative flex h-full min-h-0 shrink-0 flex-col overflow-hidden bg-(--sidebar) transition-[width] duration-200 ease-out motion-reduce:transition-none {collapsed ? 'w-14' : 'w-64'}"
>
  <div class="flex h-12 shrink-0 items-center gap-2 px-3">
    <span class="flex size-8 shrink-0 items-center justify-center"><Logo size={22} /></span>
    {#if !collapsed}
      <span class="min-w-0 flex-1 truncate font-heading text-base font-bold tracking-tight">Airlock</span>
      <Button variant="ghost" size="icon-sm" aria-label="Collapse sidebar" onclick={toggle}>
        <SidebarMinimalisticIcon size={18} />
      </Button>
    {/if}
  </div>

  <div class="flex flex-col gap-0.5 px-2 pt-1">
    <button
      type="button"
      class={row}
      data-active={onChat && chat.conversationId === null}
      aria-label="New chat"
      title={collapsed ? "New chat" : undefined}
      onclick={newChat}
    >
      <span class="flex size-5 shrink-0 items-center justify-center"><PenNewSquareIcon size={18} /></span>
      {#if !collapsed}<span class="truncate">New chat</span>{/if}
    </button>

    {#each nav as item (item.href)}
      <a
        href={item.href}
        class={row}
        data-active={page.url.pathname === item.href}
        aria-current={page.url.pathname === item.href ? "page" : undefined}
        aria-label={item.label}
        title={collapsed ? item.label : undefined}
      >
        <span class="flex size-5 shrink-0 items-center justify-center">{@render item.icon()}</span>
        {#if !collapsed}<span class="truncate">{item.label}</span>{/if}
      </a>
    {/each}
  </div>

  {#if collapsed}
    <div class="mt-auto flex flex-col items-center gap-1 px-2 pb-3">
      <Button variant="ghost" size="icon-sm" aria-label="Expand sidebar" onclick={toggle}>
        <SidebarMinimalisticIcon size={18} />
      </Button>
      <span class="flex size-8 items-center justify-center" title={status.label} role="img" aria-label={status.label}>
        <span class="size-2 rounded-full {status.dot}"></span>
      </span>
    </div>
  {:else}
    <div class="mt-4 flex min-h-0 flex-1 flex-col">
      <div class="flex items-center justify-between px-5 pb-1">
        <h2 class="text-xs font-medium text-muted-foreground">Chats</h2>
      </div>

      {#if chat.conversations.length >= SEARCH_FROM}
        <div class="px-3 pb-2">
          <Input class="h-8" placeholder="Search chats" aria-label="Search chats" bind:value={query} />
        </div>
      {/if}

      <!-- A direct flex child, so the list shrinks and the footer below can never be pushed out. -->
      <ScrollArea maxHeight="100%" scrollbar="hover" class="mx-2 min-h-0 flex-1">
        <div>
          {#if !chat.listLoaded}
            <div class="flex flex-col gap-1 px-1 pt-1" role="status" aria-label="Loading chats">
              {#each [64, 48, 72, 56] as w (w)}
                <Skeleton class="h-8 rounded-lg" style="width: {w}%" />
              {/each}
            </div>
          {:else if chat.conversations.length === 0}
            <div class="flex items-start gap-2.5 px-3 pt-2 text-sm text-muted-foreground">
              <ChatLineIcon size={18} class="mt-0.5 shrink-0" />
              <p class="text-pretty">Your chats show up here. Ask something to start one.</p>
            </div>
          {:else if groups.length === 0}
            <p class="px-3 pt-2 text-sm text-muted-foreground">No chats match "{query}".</p>
          {:else}
            {#each groups as group (group.label)}
              <section class="pb-2">
                <h3 class="px-3 pt-2 pb-1 text-xs font-medium text-muted-foreground/80">{group.label}</h3>
                <ul class="flex flex-col gap-0.5">
                  {#each group.items as c (c.id)}
                    <li class="group/chat relative">
                      {#if renamingId === c.id}
                        <Input
                          class="h-8"
                          aria-label="Chat title"
                          bind:value={renameText}
                          autofocus
                          onkeydown={(e) => {
                            if (e.key === "Enter") void commitRename();
                            if (e.key === "Escape") renamingId = null;
                          }}
                          onblur={() => void commitRename()}
                        />
                      {:else}
                        <button
                          type="button"
                          class="{row} pr-9"
                          data-active={onChat && chat.conversationId === c.id}
                          title={c.title}
                          onclick={() => openChat(c.id)}
                        >
                          <HoverMarquee text={c.title} class="flex-1 text-start" />
                        </button>
                        <div
                          class="absolute inset-y-0 right-1 flex items-center opacity-0 transition-opacity duration-100 group-focus-within/chat:opacity-100 group-hover/chat:opacity-100 has-[[data-state=open]]:opacity-100"
                        >
                          <DropdownMenu>
                            <DropdownMenuTrigger>
                              {#snippet child({ props })}
                                <Button {...props} variant="ghost" size="icon-sm" aria-label={`Actions for ${c.title}`}>
                                  <MenuDotsIcon size={16} />
                                </Button>
                              {/snippet}
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuItem onclick={() => startRename(c.id, c.title)}>Rename</DropdownMenuItem>
                              <DropdownMenuItem onclick={() => askDelete(c.id, c.title)}>Delete</DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </div>
                      {/if}
                    </li>
                  {/each}
                </ul>
              </section>
            {/each}
          {/if}
        </div>
      </ScrollArea>

      <div class="flex shrink-0 items-center gap-1 p-2">
        <a
          href="/models"
          class="flex h-9 min-w-0 flex-1 items-center gap-2.5 rounded-lg bg-foreground/[0.05] px-3 text-sm outline-none transition-colors duration-100 hover:bg-foreground/[0.09] focus-visible:ring-2 focus-visible:ring-ring"
        >
          <span class="size-2 shrink-0 rounded-full {status.dot}"></span>
          <span class="truncate">{status.label}</span>
        </a>
        <DropdownMenu>
          <DropdownMenuTrigger>
            {#snippet child({ props })}
              <Button {...props} variant="ghost" size="icon-sm" aria-label="App menu">
                <MenuDotsIcon size={18} />
              </Button>
            {/snippet}
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" side="top">
            <DropdownMenuItem onclick={() => inTauri() && commands.openAppDataDir()}>
              <FolderOpenIcon size={16} />Open data folder
            </DropdownMenuItem>
            <DropdownMenuItem onclick={() => inTauri() && openUrl(APP_URL)}>
              <GlobalIcon size={16} />Website
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem onclick={() => inTauri() && exit(0)}>
              <PowerIcon size={16} />Quit Airlock
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  {/if}
  <!-- The rail: the whole right edge is a click target for collapse and expand. -->
  <button
    type="button"
    aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
    title={collapsed ? "Expand sidebar (Ctrl+B)" : "Collapse sidebar (Ctrl+B)"}
    tabindex="-1"
    onclick={toggle}
    class="absolute inset-y-0 right-0 z-10 w-2 outline-none focus-visible:ring-2 focus-visible:ring-ring after:absolute after:inset-y-0 after:right-0 after:w-px after:bg-transparent after:transition-colors after:duration-100 hover:after:bg-foreground/20 {collapsed ? 'cursor-e-resize' : 'cursor-w-resize'}"
  ></button>
</aside>

<DeleteChatDialog bind:open={deleteOpen} id={deleting?.id ?? null} title={deleting?.title ?? "This chat"} />
