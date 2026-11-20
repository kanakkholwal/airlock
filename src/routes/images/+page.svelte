<script lang="ts">
  import PageHeader from "#lib/components/app/page-header.svelte";
  import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "#lib/components/ui/empty/index.js";
</script>

<main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-16">
  <PageHeader title="Images" />
  <Empty>
    <EmptyHeader>
      <EmptyTitle>No images yet</EmptyTitle>
      <EmptyDescription>Image generation arrives in Stage 3.</EmptyDescription>
    </EmptyHeader>
  </Empty>
</main>
