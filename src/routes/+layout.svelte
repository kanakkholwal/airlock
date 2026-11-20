<script lang="ts">
  import "../app.css";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { general } from "#lib/general-settings.svelte.js";
  import { llm } from "#lib/llm.svelte.js";
  import AppHeader from "#lib/components/app/app-header.svelte";
  import AppSidebar from "#lib/components/app/app-sidebar.svelte";
  import { ErrorBoundary } from "#lib/components/ui/error-boundary/index.js";
  import { Toaster } from "#lib/components/ui/toast/index.js";

  let { children } = $props();

  // The recording overlay is its own window with its own route, so it gets no app shell.
  const isOverlay = $derived(page.url.pathname.startsWith("/overlay"));

  onMount(() => {
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
    }
  });
</script>

{#if isOverlay}
  {@render children()}
{:else}
<div class="flex h-dvh bg-background text-foreground">
  <AppSidebar />
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
{/if}
