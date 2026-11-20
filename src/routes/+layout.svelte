<script lang="ts">
  import "../app.css";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import SpeedTest from "#lib/components/app/speed-test.svelte";
  import { llm } from "#lib/llm.svelte.js";
  import { SidebarNav } from "#lib/components/ui/sidebar-nav/index.js";
  import {
    ChatLineIcon,
    CpuIcon,
    GalleryIcon,
    MicrophoneIcon,
    SettingsIcon,
  } from "@solar-icons/svelte/linear";

  let { children } = $props();

  onMount(() => llm.init());

  const navItems = [
    { key: "chat", label: "Chat", icon: chat },
    { key: "dictation", label: "Dictation", icon: dictation },
    { key: "images", label: "Images", icon: images },
    { key: "models", label: "Models", icon: models },
    { key: "settings", label: "Settings", icon: settings },
  ];
  const routes: Record<string, string> = {
    chat: "/",
    dictation: "/dictation",
    images: "/images",
    models: "/models",
    settings: "/settings",
  };
  const active = $derived(
    navItems.find((n) => routes[n.key] === page.url.pathname)?.key ?? "chat",
  );
</script>

{#snippet chat()}<ChatLineIcon size={18} />{/snippet}
{#snippet dictation()}<MicrophoneIcon size={18} />{/snippet}
{#snippet images()}<GalleryIcon size={18} />{/snippet}
{#snippet models()}<CpuIcon size={18} />{/snippet}
{#snippet settings()}<SettingsIcon size={18} />{/snippet}

<div class="flex h-dvh bg-background text-foreground">
  <SidebarNav
    fill
    workspace={{ name: "Airlock", monogram: "A" }}
    {navItems}
    recents={[]}
    activeNav={active}
    onNavigate={(key) => goto(routes[key])}
  />
  <div class="flex min-w-0 flex-1 flex-col">
    <header class="flex h-12 shrink-0 items-center justify-end px-4">
      <SpeedTest run={llm.speedRunner} available={["chat"]} />
    </header>
    <div class="min-h-0 flex-1 overflow-y-auto">
      {@render children()}
    </div>
  </div>
</div>
