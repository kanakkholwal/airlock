<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { general } from "#lib/general-settings.svelte.js";
  import { commands } from "../bindings";
  import { inTauri } from "#lib/env.js";
  import { llm } from "#lib/llm.svelte.js";
  import { memory } from "#lib/memory-settings.svelte.js";
  import { speaker } from "#lib/speak.svelte.js";
  import { speech } from "#lib/speech.svelte.js";
  import { installUiUnlock, unlockPage } from "#lib/unlock-ui.js";
  import AppHeader from "#lib/components/app/app-header.svelte";
  import AppSidebar from "#lib/components/app/app-sidebar.svelte";
  import LoadedModels from "#lib/components/app/loaded-models.svelte";
  import VoiceControl from "#lib/components/app/voice-control.svelte";
  import { ErrorBoundary } from "#lib/components/ui/error-boundary/index.js";
  import { Toaster } from "#lib/components/ui/toast/index.js";
  import { TooltipProvider } from "#lib/components/ui/tooltip/index.js";

  let { children } = $props();

  // The recording overlay is its own window with its own route, so it gets no app shell.
  const isOverlay = $derived(page.url.pathname.startsWith("/overlay"));

  onMount(() => {
    const removeUnlock = installUiUnlock();
    const splash = document.getElementById("splash");
    if (splash) {
      splash.setAttribute("data-leaving", "");
      splash.addEventListener("transitionend", () => splash.remove(), { once: true });
      // Reduced motion has no transition, so transitionend never fires.
      setTimeout(() => splash.remove(), 400);
    }
    if (!isOverlay) {
      void llm.init();
      void general.load();
      // The shortcut and paste engine must start with the app, not when a page opens.
      void speech.init();
      speaker.init();
      void memory.load();
      // Tell the engine when the window is hidden, so the chat model can free its memory sooner.
      if (inTauri()) {
        const report = () => void commands.llmSetWindowHidden(document.hidden);
        document.addEventListener("visibilitychange", report);
        report();
      }
    }
    return removeUnlock;
  });
</script>

{#if isOverlay}
  {@render children()}
{:else}
<TooltipProvider>
<div class="flex h-dvh bg-background text-foreground">
  <AppSidebar />
  <div class="relative flex min-w-0 flex-1 flex-col">
    <!-- The chat header carries it inline so the row aligns; other pages pin it top right. -->
    {#if page.url.pathname !== "/"}
      <div class="absolute top-1.5 right-4 z-20"><LoadedModels /></div>
    {/if}
    {#if page.url.pathname !== "/voice"}<VoiceControl />{/if}
    {#if page.url.pathname === "/"}
      <AppHeader />
    {/if}
    <div class="min-h-0 flex-1">
      <ErrorBoundary
        onerror={unlockPage}
        onreset={unlockPage}
        labels={{ title: "This screen hit a problem", description: "Your chats and models are safe. Try again." }}
      >
        {@render children()}
      </ErrorBoundary>
    </div>
  </div>
</div>

<Toaster />
</TooltipProvider>
{/if}
