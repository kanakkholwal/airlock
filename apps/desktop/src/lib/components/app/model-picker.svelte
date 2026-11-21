<script lang="ts" module>
  export type PickerModel = {
    id: string;
    name: string;
    provider?: string;
    description?: string;
    /** Small trailing text on the row, such as a size. */
    meta?: string;
    /** 0 to 5, shown as a meter in the footer. */
    speed?: number;
    intelligence?: number;
  };
</script>

<script lang="ts">
  import { tick, type Snippet } from "svelte";
  import { CheckReadIcon, MagnifierIcon, SortVerticalIcon } from "@solar-icons/svelte/linear";
  import { cn } from "#lib/cn.js";
  import { groupByProvider } from "#lib/model-provider.js";

  // Decision (product): a pill that opens into a searchable list grouped by maker. The panel is
  // laid out in place and a pill-shaped window grows around the chosen row; closing folds the
  // row back onto the pill. Port of a React picker; a candidate for Baby UI.
  let {
    models,
    value,
    onchange,
    searchable = true,
    icon,
    footer,
    class: className,
  }: {
    models: PickerModel[];
    value?: string;
    onchange?: (id: string) => void;
    searchable?: boolean;
    /** Replaces the default chip icon on the pill and each row. */
    icon?: Snippet<[PickerModel]>;
    footer?: Snippet;
    class?: string;
  } = $props();

  type Phase = "closed" | "open" | "closing";
  type Point = { x: number; y: number };

  const WIDTH = 320;
  const GAP = 8;
  const RADIUS = 16;
  const EDGE = 8;
  const METER = 5;
  const OPEN_MS = 420;
  const CLOSE_MS = 340;
  const OPEN_EASE = "cubic-bezier(0.2, 0.9, 0.3, 1.1)";
  const CLOSE_EASE = "cubic-bezier(0.4, 0, 0.2, 1)";
  const uid = $props.id();

  let phase = $state<Phase>("closed");
  let query = $state("");
  let active = $state(0);
  let foldTo = $state<string | undefined>();
  let picked = $state<string | undefined>();
  let hl = $state({ y: 0, h: 0, o: 0, placed: false });

  let root = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  let triggerIcon = $state<HTMLSpanElement>();
  let surface = $state<HTMLDivElement>();
  let content = $state<HTMLDivElement>();
  let list = $state<HTMLDivElement>();
  let input = $state<HTMLInputElement>();
  let place: Point = { x: 0, y: 0 };
  // Near the bottom of the window the panel sits above the pill; otherwise it grows around the chosen row.
  let above = false;
  let height = 0;
  let running: Animation[] = [];

  const open = $derived(phase === "open");
  const selected = $derived(models.find((m) => m.id === value) ?? models[0]);
  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? models.filter((m) => `${m.name} ${m.provider ?? ""}`.toLowerCase().includes(q)) : models;
  });
  const groups = $derived(groupByProvider(filtered));
  // Flat order follows the groups so the arrow keys walk what is on screen.
  const flat = $derived(groups.flatMap((g) => g.items));
  const current = $derived(flat[active]);
  const detailed = $derived(models.some((m) => m.description || m.speed || m.intelligence));
  const reduced = () => matchMedia("(prefers-reduced-motion: reduce)").matches;
  // A pick already flies the row into place, so only outside changes roll the name.
  const roll = $derived(selected ? selected.id !== picked : false);

  // Letters keep their identity by occurrence, so shared letters between two names stay put.
  const letters = $derived.by(() => {
    const seen: Record<string, number> = {};
    return [...(selected?.name ?? "")].map((char) => {
      seen[char] = (seen[char] ?? 0) + 1;
      return { char, key: `${char}${seen[char]}` };
    });
  });

  function stopRunning() {
    for (const a of running) a.cancel();
    running = [];
  }

  function pillWindow(at: Point, pillW: number, pillH: number) {
    const top = -at.y;
    const left = -at.x;
    const right = WIDTH - left - pillW;
    const bottom = height - top - pillH;
    return `inset(${top}px ${right}px ${bottom}px ${left}px round ${pillH / 2}px)`;
  }
  const FULL = `inset(0px 0px 0px 0px round ${RADIUS}px)`;

  // The content offset that puts a row's icon and centre exactly on the pill's.
  function alignTo(row: Element | null | undefined, pillH: number): Point | null {
    const rowIcon = row?.querySelector('[data-slot="model-picker-icon"]');
    if (!row || !rowIcon || !root || !content || !triggerIcon) return null;
    const rootRect = root.getBoundingClientRect();
    const scale = rootRect.width / root.offsetWidth || 1;
    const box = content.getBoundingClientRect();
    const rowRect = row.getBoundingClientRect();
    return {
      x: (triggerIcon.getBoundingClientRect().left - rootRect.left - (rowIcon.getBoundingClientRect().left - box.left)) / scale,
      y: pillH / 2 - (rowRect.top + rowRect.height / 2 - box.top) / scale,
    };
  }

  function runOpen() {
    if (!root || !trigger || !surface || !content || !list) return;
    const pillW = trigger.offsetWidth;
    const pillH = trigger.offsetHeight;
    height = content.offsetHeight;
    const rootRect = root.getBoundingClientRect();
    const scale = rootRect.width / root.offsetWidth || 1;

    const row = list.querySelector<HTMLElement>('[aria-selected="true"]');
    if (row && list.scrollHeight > list.clientHeight) {
      list.scrollTop = row.offsetTop - list.clientHeight / 2 + row.offsetHeight / 2;
    }
    above = rootRect.top > innerHeight / 2;
    const aligned = above ? { x: 0, y: -(height + GAP) } : (alignTo(row, pillH) ?? { x: 0, y: 0 });
    // Keep the whole panel inside the window.
    place = {
      x: Math.max((EDGE - rootRect.left) / scale, Math.min(aligned.x, (innerWidth - EDGE - rootRect.left) / scale - WIDTH)),
      y: Math.max((EDGE - rootRect.top) / scale, Math.min(aligned.y, (innerHeight - EDGE - rootRect.top) / scale - height)),
    };
    content.style.transform = `translate(${place.x}px, ${place.y}px)`;
    (searchable ? input : list)?.focus({ preventScroll: true });

    const settle = () => {
      if (!content || !surface) return;
      content.style.clipPath = FULL;
      Object.assign(surface.style, {
        left: `${place.x}px`,
        top: `${place.y}px`,
        width: `${WIDTH}px`,
        height: `${height}px`,
        borderRadius: `${RADIUS}px`,
        opacity: "1",
      });
    };
    if (reduced()) return settle();

    content.style.clipPath = pillWindow(place, pillW, pillH);
    const to = { easing: OPEN_EASE, duration: OPEN_MS, fill: "forwards" as const };
    running = [
      content.animate([{ clipPath: pillWindow(place, pillW, pillH) }, { clipPath: FULL }], to),
      surface.animate(
        [
          { left: "0px", top: "0px", width: `${pillW}px`, height: `${pillH}px`, borderRadius: `${pillH / 2}px`, opacity: 0 },
          { left: `${place.x}px`, top: `${place.y}px`, width: `${WIDTH}px`, height: `${height}px`, borderRadius: `${RADIUS}px`, opacity: 1 },
        ],
        to,
      ),
    ];
    running[1].finished.then(settle).catch(() => {});
  }

  function runClose() {
    if (!trigger || !surface || !content || !list) {
      phase = "closed";
      return;
    }
    const pillW = trigger.offsetWidth;
    const pillH = trigger.offsetHeight;
    const from = place;
    const row = foldTo ? list.querySelector(`[data-id="${CSS.escape(foldTo)}"]`) : null;
    const rowRect = row?.getBoundingClientRect();
    const listRect = list.getBoundingClientRect();
    // A row scrolled out of view would fold onto an empty window, so fall back to a fade.
    const visible = rowRect && rowRect.top >= listRect.top - 1 && rowRect.bottom <= listRect.bottom + 1;
    const aligned = visible && !above ? alignTo(row, pillH) : null;
    const to = aligned ?? from;
    stopRunning();
    const opts = { easing: CLOSE_EASE, duration: CLOSE_MS, fill: "forwards" as const };
    running = [
      content.animate(
        [
          { transform: `translate(${from.x}px, ${from.y}px)`, clipPath: FULL, opacity: 1 },
          { transform: `translate(${to.x}px, ${to.y}px)`, clipPath: pillWindow(to, pillW, pillH), opacity: aligned ? 1 : 0 },
        ],
        opts,
      ),
      surface.animate(
        [
          { left: `${from.x}px`, top: `${from.y}px`, width: `${WIDTH}px`, height: `${height}px`, borderRadius: `${RADIUS}px`, opacity: 1 },
          { left: "0px", top: "0px", width: `${pillW}px`, height: `${pillH}px`, borderRadius: `${pillH / 2}px`, opacity: 0 },
        ],
        opts,
      ),
    ];
    const mine = running;
    running[0].finished
      .then(() => {
        if (running === mine) phase = "closed";
      })
      .catch(() => {});
  }

  async function show() {
    if (phase !== "closed") return;
    query = "";
    active = Math.max(0, models.findIndex((m) => m.id === selected?.id));
    hl = { y: 0, h: 0, o: 0, placed: false };
    phase = "open";
    await tick();
    runOpen();
  }

  function close(to: string | undefined, refocus: "keyboard" | "pointer" | false) {
    if (phase !== "open") return;
    foldTo = to;
    if (refocus) trigger?.focus({ preventScroll: true, focusVisible: refocus === "keyboard" } as FocusOptions);
    if (reduced()) {
      phase = "closed";
      return;
    }
    phase = "closing";
    runClose();
  }

  function choose(model: PickerModel, via: "keyboard" | "pointer") {
    if (model.id !== selected?.id) onchange?.(model.id);
    picked = model.id;
    close(model.id, via);
  }

  function onKey(e: KeyboardEvent) {
    const n = flat.length;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (n) active = (active + (e.key === "ArrowDown" ? 1 : -1) + n) % n;
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      active = e.key === "Home" ? 0 : Math.max(0, n - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (current) choose(current, "keyboard");
    } else if (e.key === "Escape") {
      e.preventDefault();
      close(selected?.id, "keyboard");
    } else if (e.key === "Tab") {
      close(selected?.id, false);
    }
  }

  // Outside clicks and resizes fold the panel away.
  $effect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => {
      if (!root?.contains(e.target as Node)) close(selected?.id, false);
    };
    const resize = () => close(selected?.id, false);
    document.addEventListener("pointerdown", outside);
    addEventListener("resize", resize);
    return () => {
      document.removeEventListener("pointerdown", outside);
      removeEventListener("resize", resize);
    };
  });

  // Filtering changes the panel height, so the surface follows it.
  $effect(() => {
    if (!open || !content || !surface) return;
    const el = content;
    const observer = new ResizeObserver(() => {
      const next = el.offsetHeight;
      if (next === height || !surface) return;
      height = next;
      if (reduced()) surface.style.height = `${next}px`;
      else {
        surface.animate([{ height: surface.style.height }, { height: `${next}px` }], { duration: 220, easing: CLOSE_EASE });
        surface.style.height = `${next}px`;
      }
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  // The highlight slides to the active row from layout offsets, so zoom cannot throw it off.
  $effect(() => {
    active;
    filtered;
    if (phase === "closed" || !list) return;
    const row = list.querySelector<HTMLElement>(`[data-index="${active}"]`);
    // Fields are written one by one: replacing the object would make this effect re-trigger itself.
    if (!row || phase === "closing") {
      hl.o = 0;
      return;
    }
    hl.y = row.offsetTop;
    hl.h = row.offsetHeight;
    hl.o = 1;
    hl.placed = true;
    // Keep the keyboard-active row in view.
    if (!open) return;
    if (row.offsetTop < list.scrollTop) list.scrollTop = row.offsetTop - 6;
    else if (row.offsetTop + row.offsetHeight > list.scrollTop + list.clientHeight)
      list.scrollTop = row.offsetTop + row.offsetHeight - list.clientHeight + 6;
  });

  $effect(() => () => stopRunning());

  const indexOf = (m: PickerModel) => flat.indexOf(m);
</script>

{#snippet chip(model: PickerModel)}
  {#if icon}
    {@render icon(model)}
  {:else}
    <span class="grid size-[18px] place-items-center rounded-md bg-muted text-[10px] font-semibold text-foreground/70">
      {(model.provider ?? model.name).charAt(0).toUpperCase()}
    </span>
  {/if}
{/snippet}

{#snippet meter(label: string, v: number)}
  <span class="flex items-center gap-2">
    <span>{label}</span>
    <span class="flex items-center gap-[3px]" role="img" aria-label={`${v} of ${METER}`}>
      {#each Array(METER) as _, i (i)}
        <span
          class={cn("h-2.5 w-[3px] rounded-full bg-current transition-opacity duration-300", i < v ? "opacity-80" : "opacity-15")}
          style:transition-delay="{i * 30}ms"
        ></span>
      {/each}
    </span>
  </span>
{/snippet}

{#if selected}
  <div bind:this={root} class={cn("relative inline-flex", className)}>
    <button
      bind:this={trigger}
      type="button"
      aria-haspopup="listbox"
      aria-expanded={open}
      aria-controls={open ? `${uid}-list` : undefined}
      aria-label={`Model: ${selected.name}`}
      class={cn(
        "flex h-8 cursor-pointer items-center gap-2 rounded-full pr-2 pl-2.5 text-sm font-medium outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/40",
        phase !== "closed" && "opacity-0",
      )}
      onclick={() => (open ? close(selected.id, "pointer") : void show())}
      onkeydown={(e) => {
        if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          e.preventDefault();
          void show();
        }
      }}
    >
      <span bind:this={triggerIcon} class="grid size-5 place-items-center text-muted-foreground">{@render chip(selected)}</span>
      <span aria-hidden="true" class="relative inline-flex max-w-44 overflow-hidden whitespace-pre">
        {#key selected.name}
          {#each letters as l, i (l.key)}
            <span class={roll && !reduced() ? "roll-in" : ""} style:animation-delay="{i * 14}ms">{l.char}</span>
          {/each}
        {/key}
      </span>
      <SortVerticalIcon size={14} class={cn("text-muted-foreground transition-opacity duration-200", phase !== "closed" ? "opacity-0" : "opacity-60")} />
    </button>

    {#if phase !== "closed"}
      <div
        bind:this={surface}
        aria-hidden="true"
        class="pointer-events-none absolute top-0 left-0 z-50 bg-popover opacity-0 shadow-(--overlay-shadow)"
      ></div>
      <div
        bind:this={content}
        role="presentation"
        class={cn("absolute top-0 left-0 z-50 flex flex-col select-none [clip-path:inset(50%)]", phase === "closing" && "pointer-events-none")}
        style:width="{WIDTH}px"
        onkeydown={onKey}
      >
        {#if searchable}
          <div class="flex h-11 items-center gap-2.5 border-b border-border/60 px-3.5 text-muted-foreground">
            <MagnifierIcon size={16} class="shrink-0" />
            <input
              bind:this={input}
              bind:value={query}
              oninput={() => (active = 0)}
              placeholder="Search models"
              role="combobox"
              aria-expanded={open}
              aria-controls={`${uid}-list`}
              aria-activedescendant={current ? `${uid}-opt-${active}` : undefined}
              aria-label="Search models"
              class="h-full min-w-0 flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground"
            />
          </div>
        {/if}

        <div
          bind:this={list}
          id={`${uid}-list`}
          role="listbox"
          aria-label="Models"
          tabindex={searchable ? undefined : -1}
          class="relative max-h-80 overflow-y-auto px-1.5 pt-0 pb-2 outline-none [scrollbar-width:none]"
        >
          <div
            aria-hidden="true"
            class="pointer-events-none absolute right-2 left-2 rounded-xl bg-accent"
            style:top="0"
            style:height="{hl.h}px"
            style:opacity={hl.o}
            style:transform="translateY({hl.y}px)"
            style:transition={hl.placed && !reduced() ? "transform 180ms cubic-bezier(0.2,0.9,0.3,1), height 180ms, opacity 150ms" : "none"}
          ></div>
          {#each groups as group (group.provider)}
            <div role="group" aria-label={group.provider}>
              <div class="px-3 pt-3 pb-1.5 text-xs font-semibold text-muted-foreground/80">{group.provider}</div>
              {#each group.items as model (model.id)}
                {@const i = indexOf(model)}
                <div
                  id={`${uid}-opt-${i}`}
                  role="option"
                  aria-selected={model.id === selected.id}
                  tabindex="-1"
                  data-id={model.id}
                  data-index={i}
                  class={cn(
                    "relative flex h-10 cursor-pointer items-center gap-3 rounded-xl px-3 text-sm font-semibold transition-colors",
                    i === active || model.id === foldTo ? "text-foreground" : "text-foreground/70",
                  )}
                  onpointermove={() => (active = i)}
                  onmousedown={(e) => e.preventDefault()}
                  onclick={() => choose(model, "pointer")}
                  onkeydown={(e) => e.key === "Enter" && choose(model, "keyboard")}
                >
                  <span data-slot="model-picker-icon" class="relative grid size-[18px] shrink-0 place-items-center text-muted-foreground">
                    {@render chip(model)}
                  </span>
                  <span class="relative min-w-0 flex-1 truncate">{model.name}</span>
                  {#if model.meta}<span class="relative shrink-0 text-xs font-normal text-muted-foreground">{model.meta}</span>{/if}
                  {#if model.id === selected.id}
                    <CheckReadIcon size={14} class={cn("relative shrink-0 opacity-60 transition-opacity duration-150", phase === "closing" && "opacity-0")} />
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
          {#if !flat.length}
            <p class="px-3 py-8 text-center text-sm text-muted-foreground">No models match</p>
          {/if}
        </div>

        {#if detailed && current}
          <div class="flex flex-col gap-2 border-t border-border/60 px-3.5 py-3 text-xs text-muted-foreground">
            {#key current.id}
              <span class="swap-in line-clamp-2 text-pretty text-foreground/80">{current.description ?? current.name}</span>
            {/key}
            {#if current.speed !== undefined || current.intelligence !== undefined}
              <span class="flex items-center gap-5">
                {#if current.speed !== undefined}{@render meter("Speed", current.speed)}{/if}
                {#if current.intelligence !== undefined}{@render meter("Intelligence", current.intelligence)}{/if}
              </span>
            {/if}
          </div>
        {/if}

        {#if footer}<div class="border-t border-border/60 p-1.5">{@render footer()}</div>{/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .roll-in {
    display: inline-block;
    animation: picker-roll 420ms cubic-bezier(0.2, 0.9, 0.3, 1.1) backwards;
  }
  .swap-in {
    animation: picker-swap 300ms cubic-bezier(0.2, 0.9, 0.3, 1.1) backwards;
  }
  @keyframes picker-roll {
    from { opacity: 0; transform: translateY(0.7em); filter: blur(3px); }
  }
  @keyframes picker-swap {
    from { opacity: 0; transform: translateY(4px); filter: blur(3px); }
  }
  @media (prefers-reduced-motion: reduce) {
    .roll-in, .swap-in { animation: none; }
  }
</style>
