<script lang="ts">
  import { onMount } from "svelte";
  import { LiveTranscript, LiveTranscriptBody, LiveTranscriptStatus } from "#lib/components/agents/live-transcript/index.js";
  import { RevealText } from "#lib/components/text/reveal-text/index.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import Buddies from "./Buddies.svelte";
  import ChatDemo from "./ChatDemo.svelte";
  import ImageDemo from "./ImageDemo.svelte";

  // Decision (product): each tab runs the component the app itself uses for that job, on sample
  // text, so the reader sees how it behaves instead of reading about it. It says it is a preview.
  // Read aloud is left out: it is not a strong enough moment on screen yet.
  const tabs = [
    { id: "chat", label: "Chat", caption: "Answers from a model on your disk." },
    { id: "dictation", label: "Dictation", caption: "Hold a key in any app and talk." },
    { id: "images", label: "Images", caption: "Describe it. Your graphics card draws it." },
  ];
  let tab = $state("chat");
  const caption = $derived(tabs.find((t) => t.id === tab)?.caption);

  // Dictation: words arrive, and the last two stay tentative until the next word settles them.
  const spoken = "Let's move the review to Thursday afternoon and send the notes to Priya".split(" ");
  let heard = $state(1);
  onMount(() => {
    const id = setInterval(() => {
      if (tab === "dictation") heard = heard >= spoken.length + 3 ? 1 : heard + 1;
    }, 420);
    return () => clearInterval(id);
  });
  const shown = $derived(Math.min(heard, spoken.length));
  const committed = $derived(spoken.slice(0, Math.max(0, shown - 2)).join(" "));
  const tentative = $derived(spoken.slice(Math.max(0, shown - 2), shown).join(" "));
</script>

<section id="jobs" class="relative mx-auto w-full max-w-5xl scroll-mt-24 px-6 py-28" aria-labelledby="jobs-title">
  <Buddies section="see-it-work" spots={["absolute top-28 -left-6 hidden w-20 xl:block", "absolute -right-8 bottom-24 hidden w-24 xl:block"]} />
  <div class="text-center">
    <h2 id="jobs-title" class="text-4xl sm:text-5xl"><RevealText text="See it work" trigger="view" staggerMs={90} /></h2>
  </div>

  <div class="mt-10 flex justify-center">
    <Tabs bind:value={tab} variant="segment">
      <TabsList>
        {#each tabs as t (t.id)}<TabsTrigger value={t.id}>{t.label}</TabsTrigger>{/each}
      </TabsList>
    </Tabs>
  </div>

  <div class="mt-8 rounded-2xl bg-card p-3 shadow-(--surface-shadow) sm:p-4">
    <div class="flex items-center justify-between px-2 pb-3 text-xs text-muted-foreground">
      <span>Live preview</span><span>Sample text</span>
    </div>
    <div class="flex min-h-[26rem] flex-col justify-center rounded-xl bg-background p-5 sm:p-10">
      {#key tab}
        <div class="mx-auto w-full max-w-2xl motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
          {#if tab === "chat"}
            <ChatDemo />
          {:else if tab === "dictation"}
            <LiveTranscript {committed} {tentative} session={tab}>
              <LiveTranscriptBody>Say something to start.</LiveTranscriptBody>
              <LiveTranscriptStatus>Listening</LiveTranscriptStatus>
            </LiveTranscript>
          {:else}
            <ImageDemo />
          {/if}
        </div>
      {/key}
    </div>
  </div>

  <p class="mt-5 text-center text-muted-foreground" aria-live="polite">{caption}</p>
</section>
