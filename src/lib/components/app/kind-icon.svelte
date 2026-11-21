<script lang="ts">
  import {
    DocumentTextIcon,
    FolderWithFilesIcon,
    GalleryIcon,
    MicrophoneIcon,
    NotesIcon,
    TextFieldIcon,
  } from "@solar-icons/svelte/linear";

  // One icon per kind of library item, so a recording looks like a recording everywhere.
  let { kind, size = 16, class: classProp = "" }: { kind: string; size?: number; class?: string } = $props();
</script>

{#if kind === "recording"}
  <MicrophoneIcon {size} class={classProp} />
{:else if kind === "transcript"}
  <DocumentTextIcon {size} class={classProp} />
{:else if kind === "note"}
  <NotesIcon {size} class={classProp} />
{:else if kind === "summary"}
  <TextFieldIcon {size} class={classProp} />
{:else if kind === "image"}
  <GalleryIcon {size} class={classProp} />
{:else}
  <FolderWithFilesIcon {size} class={classProp} />
{/if}
