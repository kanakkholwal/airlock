<script lang="ts">
  import "../app.css";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { exit } from "@tauri-apps/plugin-process";
  import {
    ChatLineIcon,
    CpuIcon,
    FolderOpenIcon,
    GlobalIcon,
    MicrophoneIcon,
    PenNewSquareIcon,
    PowerIcon,
    SettingsIcon,
  } from "@solar-icons/svelte/linear";
  import { commands } from "../bindings";
  import { APP_URL } from "#lib/app-info.js";
  import { inTauri } from "#lib/env.js";
  import { chat } from "#lib/chat.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import AppHeader from "#lib/components/app/app-header.svelte";
  import Logo from "#lib/components/app/logo.svelte";
  import { ErrorBoundary } from "#lib/components/ui/error-boundary/index.js";
  import { SidebarNav } from "#lib/components/ui/sidebar-nav/index.js";
  import { Toaster } from "#lib/components/ui/toast/index.js";

  let { children } = $props();

  // Decision (product): Images is not in the nav until image generation exists. A nav item
  // for something that is not there is a dead end.
  const navItems = [
    { key: "chat", label: "Chat", icon: chatIcon },
    { key: "dictation", label: "Dictation", icon: dictationIcon },
    { key: "models", label: "Models", icon: modelsIcon },
    { key: "settings", label: "Settings", icon: settingsIcon },
  ];
  const routes: Record<string, string> = {
    chat: "/",
    dictation: "/dictation",
    models: "/models",
    settings: "/settings",
  };

  onMount(() => {
    const splash = document.getElementById("splash");
    if (splash) {
      splash.setAttribute("data-leaving", "");
      splash.addEventListener("transitionend", () => splash.remove(), { once: true });
      // Reduced motion has no transition, so transitionend never fires.
      setTimeout(() => splash.remove(), 400);
    }
    void llm.init();
  });

  // Decision (product): the logo menu holds what the nav does not: leaving the app (it lives
  // in the tray, so closing the window is not quitting), and finding its files and website.
  const appMenu = [
    { label: "Open data folder", icon: folderIcon, onSelect: () => inTauri() && commands.openAppDataDir() },
    { label: "Website", icon: globeIcon, onSelect: () => inTauri() && openUrl(APP_URL) },
    { label: "Quit Airlock", icon: quitIcon, onSelect: () => inTauri() && exit(0) },
  ];

  const recents = $derived(chat.conversations.map((c) => ({ id: String(c.id), label: c.title })));
  const active = $derived(
    navItems.find((n) => routes[n.key] === page.url.pathname)?.key ?? "chat",
  );

  // The footer is the one always-visible answer to "can I chat right now?".
  const footer = $derived.by(() => {
    const e = llm.engine;
    if (e.state === "ready") return `${llm.current?.name ?? "Model"} ready`;
    if (e.state === "loading") return `Loading ${llm.current?.name ?? "model"}`;
    if (e.state === "failed") return "Engine problem. Open Models";
    return llm.downloaded.length > 0 ? "No model loaded" : "Set up a model";
  });
</script>

{#snippet chatIcon()}<ChatLineIcon size={18} />{/snippet}
{#snippet dictationIcon()}<MicrophoneIcon size={18} />{/snippet}
{#snippet modelsIcon()}<CpuIcon size={18} />{/snippet}
{#snippet settingsIcon()}<SettingsIcon size={18} />{/snippet}
{#snippet logo()}<Logo size={20} />{/snippet}
{#snippet newChatIcon()}<PenNewSquareIcon size={18} />{/snippet}
{#snippet folderIcon()}<FolderOpenIcon size={16} />{/snippet}
{#snippet globeIcon()}<GlobalIcon size={16} />{/snippet}
{#snippet quitIcon()}<PowerIcon size={16} />{/snippet}

<div class="flex h-dvh bg-background text-foreground">
  <SidebarNav
    fill
    {logo}
    workspace={{ name: "Airlock", monogram: "A" }}
    {navItems}
    {recents}
    {newChatIcon}
    workspaceActions={appMenu}
    activeNav={active}
    onNavigate={(key) => goto(routes[key])}
    onPick={(id) => {
      void chat.open(Number(id));
      void goto("/");
    }}
    onNewChat={() => {
      chat.newChat();
      void goto("/");
    }}
    footerLabel={footer}
    footerIcon={modelsIcon}
    onFooterClick={() => goto("/models")}
    labels={{ navigation: "Airlock navigation", noResults: "No chats match" }}
  />
  <div class="flex min-w-0 flex-1 flex-col">
    {#if page.url.pathname === "/"}
      <AppHeader />
    {/if}
    <div class="min-h-0 flex-1">
      <ErrorBoundary
        labels={{ title: "This screen hit a problem", description: "Your chats and models are safe. Try again." }}
      >
        {@render children()}
      </ErrorBoundary>
    </div>
  </div>
</div>

<Toaster />
