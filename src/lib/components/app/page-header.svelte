<script lang="ts">
  let { title, description }: { title: string; description?: string } = $props();
</script>

<header class="text-center">
  <h1 class="text-3xl text-balance">{title}</h1>
  {#if description}
    <p class="mt-2 text-muted-foreground text-pretty">{description}</p>
  {/if}
</header>
