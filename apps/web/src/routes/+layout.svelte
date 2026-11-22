<script lang="ts">
  import "../app.css";
  import "../brand.css";

  let { children } = $props();
</script>

<svelte:head>
  <title>Airlock: private AI on your computer</title>
  <meta
    name="description"
    content="Chat, dictate, transcribe, read aloud and make images with local models. Nothing leaves your machine unless you say so."
  />
</svelte:head>

{@render children()}
