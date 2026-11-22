<script lang="ts">
  import { MenuDotsIcon } from "@solar-icons/svelte/linear";
  import { onMount } from "svelte";
  import { cn } from "#lib/cn.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#lib/components/ui/collapsible/index.js";
  import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
    navigationMenuTriggerStyle,
  } from "#lib/components/ui/navigation-menu/index.js";
  import { Sheet, SheetClose, SheetContent, SheetHeader, SheetTitle } from "#lib/components/ui/sheet/index.js";
  import Logo from "./Logo.svelte";
  import { RELEASES, REPO, menus, navLinks } from "./content.js";

  // Adapted from the GlyphTeX site header: a pill inside a pill, a framed menu with one feature
  // tile and a list of links, and a sheet on small screens. Here the bar is clear at the top of
  // the page and takes its surface once you scroll, so the hero reads through it.
  const format = new Intl.NumberFormat("en", { notation: "compact" });

  let scrolled = $state(false);
  let menuOpen = $state(false);
  let stars = $state<number | null>(null);

  onMount(() => {
    const onScroll = () => (scrolled = window.scrollY > 8);
    onScroll();
    addEventListener("scroll", onScroll, { passive: true });
    // The star count is public data, fetched once; the button works without it.
    fetch("https://api.github.com/repos/kanakkholwal/airlock")
      .then((r) => (r.ok ? r.json() : null))
      .then((d) => {
        if (d && typeof d.stargazers_count === "number") stars = d.stargazers_count;
      })
      .catch(() => {});
    return () => removeEventListener("scroll", onScroll);
  });
</script>

{#snippet github()}
  <Button href={REPO} target="_blank" rel="noopener noreferrer" variant="ghost" class="gap-1.5 rounded-full! text-muted-foreground">
    <svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" aria-hidden="true">
      <path d="M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2c-3.2.7-3.87-1.37-3.87-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.04-.71.08-.7.08-.7 1.15.08 1.76 1.18 1.76 1.18 1.03 1.76 2.69 1.25 3.35.96.1-.74.4-1.25.73-1.54-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.28 1.18-3.09-.12-.29-.51-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.78 0c2.2-1.49 3.17-1.18 3.17-1.18.62 1.59.23 2.76.11 3.05.74.81 1.18 1.83 1.18 3.09 0 4.42-2.69 5.39-5.25 5.68.41.35.78 1.05.78 2.12v3.14c0 .31.21.67.8.56A11.5 11.5 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5Z" />
    </svg>
    <span class="sr-only">Airlock Studio on GitHub</span>
    {#if stars !== null && stars > 0}
      <span class="tabular-nums">{format.format(stars)}<span class="sr-only"> stars</span></span>
    {/if}
  </Button>
{/snippet}

<!-- The negative bottom margin pulls the hero up under the bar, so the bar sits on the hero. -->
<header class="sticky top-0 z-40 -mb-[5.25rem] px-3 pt-3 sm:px-5 lg:px-8">
  <div
    class="mx-auto rounded-full p-1 transition-[background-color,box-shadow,max-width] duration-500 ease-(--ease-ui-out) {scrolled
      ? 'max-w-[62rem] bg-card shadow-(--surface-shadow)'
      : 'max-w-[72rem] bg-transparent shadow-none'}"
  >
    <div
      class="grid h-14 grid-cols-[1fr_auto] items-center gap-3 rounded-full pr-2 pl-5 transition-[background-color] duration-300 ease-(--ease-ui-out) lg:grid-cols-[auto_1fr_auto] {scrolled
        ? 'bg-background'
        : 'bg-transparent'}"
    >
      <a
        href="#top"
        class="-m-1.5 flex shrink-0 items-center gap-2 rounded-md p-1.5 font-heading text-base font-bold tracking-tight outline-none focus-visible:ring-2 focus-visible:ring-ring"
        aria-label="Airlock Studio home"
      >
        <Logo size={24} />Airlock Studio
      </a>

      <div class="hidden justify-self-center lg:block">
        <NavigationMenu variant="framed" aria-label="Primary">
          <NavigationMenuList class="gap-1">
            {#each menus as menu (menu.label)}
              <NavigationMenuItem value={menu.label}>
                {@const Mark = menu.items[0].icon}
                <NavigationMenuTrigger class="rounded-full text-muted-foreground">{menu.label}</NavigationMenuTrigger>
                <NavigationMenuContent>
                  <div class="grid w-[min(40rem,calc(100vw-23rem))] grid-cols-[13rem_1fr] gap-1">
                    <a
                      href={menu.feature.href}
                      class="group/feature relative flex min-h-60 flex-col justify-between gap-6 overflow-hidden rounded-xl bg-muted p-5 text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    >
                      <span class="grid size-12 place-items-center self-start rounded-xl bg-card text-foreground shadow-(--surface-shadow) transition-transform duration-300 ease-(--ease-ui-out) group-hover/feature:-rotate-3">
                        <Mark size={24} />
                      </span>
                      <span class="block">
                        <span class="block text-lg leading-tight font-medium">{menu.feature.title}</span>
                        <span class="mt-2 block text-sm text-pretty text-muted-foreground">{menu.feature.desc}</span>
                        <span class="mt-4 inline-flex items-center gap-1.5 text-sm font-medium text-foreground">
                          {menu.feature.cta}<span aria-hidden="true">→</span>
                        </span>
                      </span>
                    </a>
                    <ul class={["grid content-start gap-x-2 gap-y-0.5 p-2", menu.items.length > 3 && "sm:grid-cols-2"]}>
                      {#each menu.items as item (item.href + item.label)}
                        <li>
                          <NavigationMenuLink href={item.href} class="flex-row items-start gap-3 rounded-xl p-3">
                            <span class="mt-0.5 grid size-8 shrink-0 place-items-center rounded-lg bg-muted text-foreground" aria-hidden="true">
                              <item.icon size={16} />
                            </span>
                            <span class="min-w-0">
                              <span class="block text-sm font-medium text-foreground">{item.label}</span>
                              <span class="mt-0.5 block text-xs text-pretty text-muted-foreground">{item.desc}</span>
                            </span>
                          </NavigationMenuLink>
                        </li>
                      {/each}
                    </ul>
                  </div>
                </NavigationMenuContent>
              </NavigationMenuItem>
            {/each}
            {#each navLinks as link (link.href)}
              <NavigationMenuItem>
                <NavigationMenuLink href={link.href} class={cn(navigationMenuTriggerStyle(), "rounded-full text-muted-foreground no-underline")}>{link.label}</NavigationMenuLink>
              </NavigationMenuItem>
            {/each}
          </NavigationMenuList>
        </NavigationMenu>
      </div>

      <div class="flex items-center justify-end gap-1">
        <span class="hidden sm:contents">{@render github()}</span>
        <Button variant="dark" href={RELEASES} class="hidden rounded-full! px-5 sm:inline-flex">Download</Button>
        <Button variant="ghost" size="icon-sm" class="lg:hidden" aria-label="Open menu" aria-expanded={menuOpen} onclick={() => (menuOpen = true)}>
          <MenuDotsIcon size={20} />
        </Button>
      </div>
    </div>
  </div>
</header>

<Sheet bind:open={menuOpen}>
  <SheetContent side="right" class="flex w-[min(20rem,85vw)] flex-col gap-0 p-0">
    <SheetHeader class="flex-row items-center justify-between px-5 py-4">
      <SheetTitle class="flex items-center gap-2 font-heading text-base font-bold"><Logo size={22} />Airlock Studio</SheetTitle>
      <SheetClose class="-mr-2" />
    </SheetHeader>
    <nav aria-label="Mobile" class="flex-1 overflow-y-auto p-3">
      {#each menus as menu (menu.label)}
        <Collapsible>
          <CollapsibleTrigger class="min-h-12 px-3 text-base">{menu.label}</CollapsibleTrigger>
          <CollapsibleContent>
            <ul class="flex flex-col pb-3 pl-3">
              {#each menu.items as item (item.href + item.label)}
                <li>
                  <a
                    href={item.href}
                    onclick={() => (menuOpen = false)}
                    class="flex min-h-11 items-center gap-3 rounded-lg px-3 text-sm text-foreground transition-colors hover:bg-foreground/[0.06]"
                  >
                    <item.icon size={16} />
                    {item.label}
                  </a>
                </li>
              {/each}
            </ul>
          </CollapsibleContent>
        </Collapsible>
      {/each}
      {#each navLinks as link (link.href)}
        <a href={link.href} onclick={() => (menuOpen = false)} class="flex min-h-12 items-center px-3 text-base">{link.label}</a>
      {/each}
    </nav>
    <div class="flex flex-col gap-2 p-4">
      <Button variant="dark" href={RELEASES} onclick={() => (menuOpen = false)}>Download</Button>
      <div class="flex items-center justify-between">{@render github()}</div>
    </div>
  </SheetContent>
</Sheet>
