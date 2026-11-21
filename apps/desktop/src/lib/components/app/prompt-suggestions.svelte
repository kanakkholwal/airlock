<script lang="ts">
  import { DocumentTextIcon, LightbulbIcon, MicrophoneIcon, PenNewSquareIcon } from "@solar-icons/svelte/linear";
  import { draft } from "#lib/draft.svelte.js";

  // Decision (product): the start screen offers a few ways in. Ones ending in @ open the mention
  // list at once, so "summarize a recording" is two clicks. They vanish as soon as you type.
  const ideas = [
    { icon: MicrophoneIcon, label: "Summarize a recording", text: "Summarize @" },
    { icon: DocumentTextIcon, label: "Explain a file", text: "Explain @" },
    { icon: PenNewSquareIcon, label: "Draft a message", text: "Help me write a message to " },
    { icon: LightbulbIcon, label: "Brainstorm ideas", text: "Give me ideas for " },
  ];
</script>

{#if !draft.text}
  <ul class="flex flex-wrap justify-center gap-2" aria-label="Ideas">
    {#each ideas as idea, i (idea.label)}
      <li
        class="motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2 motion-safe:fill-mode-backwards motion-safe:duration-300"
        style:animation-delay="{i * 40}ms"
      >
        <button
          type="button"
          class="flex h-9 items-center gap-2 rounded-full bg-card px-3.5 text-sm text-foreground/80 transition-[background-color,color,scale] duration-150 ease-out hover:bg-muted hover:text-foreground active:scale-[0.97] focus-visible:ring-2 focus-visible:ring-ring/40 focus-visible:outline-none"
          onclick={() => (draft.text = idea.text)}
        >
          <idea.icon size={16} class="text-muted-foreground" />
          {idea.label}
        </button>
      </li>
    {/each}
  </ul>
{/if}
