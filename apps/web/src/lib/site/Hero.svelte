<script lang="ts">
  import {
    ChatLineIcon,
    ClipboardIcon,
    ClockCircleIcon,
    CpuIcon,
    DocumentTextIcon,
    FolderOpenIcon,
    GalleryIcon,
    KeyboardIcon,
    LockKeyholeIcon,
    MagicWandIcon,
    ShieldCheckIcon,
    VolumeLoudIcon,
  } from "@solar-icons/svelte/linear";
  import type { Component } from "svelte";
  import {
    OrbitHero,
    OrbitHeroAction,
    OrbitHeroActions,
    OrbitHeroBadge,
    OrbitHeroContent,
    OrbitHeroDescription,
    OrbitHeroSubheading,
    OrbitHeroTitle,
  } from "#lib/components/blocks/orbit-hero/index.js";
  import { DiaText } from "#lib/components/text/dia-text/index.js";
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { RollText } from "#lib/components/text/roll-text/index.js";
  import { RELEASES, REPO } from "./content.js";

  // Decision (product): the hero is the airlock itself, a chamber with the studio's tools turning
  // around a dial. The outer ring is what you can do, the inner ring is what you can count on.
  // The component's decorative side readouts ("Models", "Tone") say nothing about the product,
  // so they are hidden below. Buttons are neutral and the brand colour is left to the glow and
  // the two swept words.
  type Orb = { label: string; icon: Component<{ size?: number }> };
  type Group = { label: string; outer: Orb[]; inner: Orb[] };

  const groups: Group[] = [
    {
      label: "Studio",
      outer: [
        { label: "Chat", icon: ChatLineIcon },
        { label: "Dictation", icon: KeyboardIcon },
        { label: "Transcribe", icon: DocumentTextIcon },
        { label: "Read aloud", icon: VolumeLoudIcon },
        { label: "Images", icon: GalleryIcon },
        { label: "Library", icon: FolderOpenIcon },
        { label: "Assistant", icon: MagicWandIcon },
        { label: "Subtitles", icon: ClockCircleIcon },
      ],
      inner: [
        { label: "Private", icon: LockKeyholeIcon },
        { label: "Open code", icon: ShieldCheckIcon },
        { label: "Your GPU", icon: CpuIcon },
        { label: "Logged", icon: ClipboardIcon },
      ],
    },
  ];

  // Ink and blue, so the rings read as the product's own colours without filling the hero with orange.
  const tones = [
    { label: "Ink", color: "var(--foreground)" },
    { label: "Blue", color: "var(--tertiary)" },
  ];
</script>

<div id="top">
  <!-- Edge to edge: the panel keeps its tint and glow but drops the inset and rounded corners, and
       leaves room at the top for the header that sits on it. -->
  <OrbitHero {groups} {tones} size="screen" class="px-0! [&>div]:mb-0! [&>div]:rounded-none! [&>div]:pt-28!">
    {#snippet item(o)}
      <span class="flex size-8 items-center justify-center" title={o.label}>
        <o.icon size={22} />
      </span>
    {/snippet}
    <OrbitHeroContent>
      <OrbitHeroBadge class="p-0">
        <a href={RELEASES} class="flex h-full items-center gap-2 rounded-full py-0.5 pr-3 pl-0.5 text-sm text-muted-foreground transition-colors hover:text-foreground">
          <span class="rounded-full bg-foreground px-2.5 py-0.5 text-xs font-medium text-background">Pre-release</span>
          Windows build is out<span aria-hidden="true">→</span>
        </a>
      </OrbitHeroBadge>
      <OrbitHeroTitle class="font-heading font-medium!">
        <RevealText text="Your AI studio" split="word" staggerMs={110} />
        <OrbitHeroSubheading>
          <DiaText
            text="on your own computer"
            colors={["var(--tertiary)", "var(--primary)", "var(--tertiary)"]}
            textColor="var(--muted-foreground)"
            delayMs={500}
            durationMs={1800}
            triggerOnView={false}
          />
        </OrbitHeroSubheading>
      </OrbitHeroTitle>
      <OrbitHeroDescription class="line-clamp-2 max-w-[38rem]!">
        Chat, dictate, read aloud and make images with models you choose. Nothing leaves your machine unless you say so.
      </OrbitHeroDescription>
      <OrbitHeroActions>
        <OrbitHeroAction variant="dark" href={RELEASES} class="rounded-full! px-6"><RollText text="Download for Windows" groupHover /></OrbitHeroAction>
        <OrbitHeroAction variant="secondary" href={REPO} class="rounded-full! px-6">View the code</OrbitHeroAction>
      </OrbitHeroActions>
    </OrbitHeroContent>
  </OrbitHero>
</div>

<style>
  /* The side readouts are decoration that names nothing in the product. */
  #top :global(.\@6xl\:flex) {
    display: none !important;
  }
</style>
