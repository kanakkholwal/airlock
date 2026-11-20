<script lang="ts">
  import type { Component } from "svelte";
  import { onMount } from "svelte";

  type Item = { id: string; label: string; icon: Component<{ size?: number | string }> };
  let { items }: { items: Item[] } = $props();

  // The section nearest the top of the page is the current one. IntersectionObserver reports
  // visibility after scroll-container clipping, so it works inside the Scroll Area.
  let current = $state("");

  onMount(() => {
    current = items[0]?.id ?? "";
    const visible = new Set<string>();
    const observer = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) visible.add(e.target.id);
          else visible.delete(e.target.id);
        }
        const first = items.find((i) => visible.has(i.id));
        if (first) current = first.id;
      },
      { rootMargin: "0px 0px -60% 0px" },
    );
    for (const i of items) {
      const el = document.getElementById(i.id);
      if (el) observer.observe(el);
    }
    return () => observer.disconnect();
  });

  function go(id: string) {
    current = id;
    document.getElementById(id)?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
</script>

<nav aria-label="Settings sections" class="sticky top-12 hidden w-48 shrink-0 flex-col gap-0.5 self-start lg:flex">
  {#each items as item (item.id)}
    <button
      type="button"
      aria-current={current === item.id ? "true" : undefined}
      data-active={current === item.id}
      onclick={() => go(item.id)}
      class="flex h-9 items-center gap-2.5 rounded-lg px-3 text-start text-sm text-muted-foreground outline-none transition-colors duration-100 hover:bg-foreground/[0.06] hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring data-[active=true]:bg-foreground/[0.09] data-[active=true]:font-medium data-[active=true]:text-foreground"
    >
      <item.icon size={18} />
      {item.label}
    </button>
  {/each}
</nav>
