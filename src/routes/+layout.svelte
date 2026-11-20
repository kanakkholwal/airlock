<script lang="ts">
  import "../app.css";
  import { SidebarNav } from "#lib/components/ui/sidebar-nav/index.js";
  import {
    ChatLineIcon,
    CpuIcon,
    GalleryIcon,
    MicrophoneIcon,
    SettingsIcon,
  } from "@solar-icons/svelte/linear";

  let { children } = $props();

  const navItems = [
    { key: "chat", label: "Chat", icon: chat },
    { key: "dictation", label: "Dictation", icon: dictation },
    { key: "images", label: "Images", icon: images },
    { key: "models", label: "Models", icon: models },
    { key: "settings", label: "Settings", icon: settings },
  ];
  let active = $state("chat");
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
    bind:activeNav={active}
  />
  <div class="min-w-0 flex-1 overflow-y-auto">
    {@render children()}
  </div>
</div>
