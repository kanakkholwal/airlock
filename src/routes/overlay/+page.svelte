<script lang="ts">
  import { onMount, tick } from "svelte";
  import { DangerCircleIcon, MicrophoneIcon } from "@solar-icons/svelte/linear";
  import { commands } from "../../bindings";
  import { noticeDisplay } from "#lib/dictation/notices.js";
  import { isNoticeState, OverlaySession } from "#lib/dictation/overlay.svelte.js";
  import "./overlay.css";

  // Decision (product): the overlay floats over other apps while you speak, so it shows only
  // what you need in the moment: that it is listening, what it heard, that it is working, or
  // why it could not. It is its own window and route, outside the app shell.
  const o = new OverlaySession();

  // 13px padding on each side, the 16px badge and its 7px gap.
  const NOTICE_PILL_CHROME = 49;

  let capEl = $state<HTMLDivElement>();
  let probeEl = $state<HTMLSpanElement>();
  let pinned = true;
  let overflowing = $state(false);
  let noticeWidth = $state<number>();

  onMount(() => {
    // Transparent window: the app's page background and the loading splash must not show.
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    document.getElementById("splash")?.remove();

    // The overlay floats over other apps, so a stray right-click must not open the webview menu.
    const noMenu = (e: Event) => e.preventDefault();
    if (!import.meta.env.DEV) window.addEventListener("contextmenu", noMenu);

    let stop: (() => void) | undefined;
    void o.start().then((s) => (stop = s));
    return () => {
      stop?.();
      window.removeEventListener("contextmenu", noMenu);
    };
  });

  // The elapsed timer starts only once microphone samples are flowing.
  $effect(() => {
    if (o.state !== "streaming" || !o.isVisible || !o.captureReady) return;
    const id = setInterval(() => (o.elapsed += 1), 1000);
    return () => clearInterval(id);
  });

  // Stick to the newest line while pinned, so reading history is not interrupted by new text.
  $effect(() => {
    void o.streamText;
    void tick().then(() => {
      if (!capEl) return;
      overflowing = capEl.scrollHeight > capEl.clientHeight + 1;
      if (pinned) capEl.scrollTop = capEl.scrollHeight;
    });
  });

  // Each fresh streaming session starts pinned to the bottom.
  $effect(() => {
    void o.session;
    pinned = true;
    overflowing = false;
  });

  // A slow model load opens the live panel with a notice, and the panel stays open afterwards.
  $effect(() => {
    if (o.modelLoadSlow && o.state === "streaming") o.loadNoticeShown = true;
  });

  const shownNotice = $derived(o.notice && isNoticeState(o.state) ? o.notice : null);
  const noticeTitle = $derived(shownNotice ? noticeDisplay(shownNotice.code).title : "");

  // CSS cannot animate to an intrinsic width, so a notice pill gets a measured one.
  $effect(() => {
    void noticeTitle;
    void o.isVisible;
    void tick().then(() => {
      if (probeEl) noticeWidth = Math.ceil(probeEl.getBoundingClientRect().width) + NOTICE_PILL_CHROME;
    });
  });

  const fmtTime = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  const transcribingLabel = $derived(o.modelLoading ? "Loading model…" : "Transcribing…");
  // One smooth 0 to 1 loudness for the meter, from the same samples the old bars used.
  const level = $derived(
    o.levels.length ? Math.min(1, Math.pow(o.levels.reduce((a, b) => a + b, 0) / o.levels.length, 0.6) * 1.8) : 0,
  );

  function onStreamScroll() {
    if (!capEl) return;
    pinned = capEl.scrollHeight - capEl.scrollTop - capEl.clientHeight <= 16;
  }

  const dismissNotice = () => {
    if (shownNotice) void commands.dismissOverlayNotice();
  };

  const noticeClass = $derived(shownNotice ? `cnotice level-${shownNotice.level}` : "");
  const liveNotice = $derived(o.state === "live_notice" ? shownNotice : null);
  const hasText = $derived(o.streamText.committed.length > 0 || o.streamText.tentative.length > 0);
  const liveWorking = $derived(!liveNotice && o.phase === "working");
  // A notice keeps the transcript; with none, the panel explains the notice.
  const noticeText = $derived(
    liveNotice && !hasText ? (noticeDisplay(liveNotice.code).explanation?.() ?? liveNotice.detail) : null,
  );
  // While listening, a slow model load opens the panel with a notice where the text would be.
  const loadingNotice = $derived(!liveNotice && o.modelLoadSlow && !liveWorking && !hasText);
  // Keep the panel open whenever there is text, even while finalizing; only fall back to the
  // small working pill when there was no text to preserve.
  const open = $derived(
    liveNotice ? hasText || !!noticeText : hasText || loadingNotice || (o.loadNoticeShown && !liveWorking),
  );
  const collapsed = $derived(liveWorking && !hasText);
  const compactWorking = $derived(o.state === "transcribing" || o.state === "processing");
  const compactLabel = $derived(o.state === "processing" ? "Processing…" : transcribingLabel);
</script>

{#snippet cancelBtn()}
  <button class="sx" aria-label="Cancel" onclick={() => commands.cancelOperation()}>
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M4 4 L12 12 M12 4 L4 12" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
    </svg>
  </button>
{/snippet}

{#snippet listeningRow(showTimer: boolean)}
  <div class="sbase">
    <MicrophoneIcon size={16} class="sicon {o.captureReady ? 'ready' : 'arming'}" />
    <span class="slabel">{o.captureReady ? "Listening" : "Starting"}</span>
    <div class="smeter" aria-hidden="true"><i style="transform: scaleX({o.captureReady ? Math.max(0.04, level) : 0.04})"></i></div>
    <div class="sbase-r">
      {#if showTimer}<span class="stimer">{fmtTime(o.elapsed)}</span>{/if}
      {@render cancelBtn()}
    </div>
  </div>
{/snippet}

{#snippet workingRow(label: string)}
  <div class="sbase">
    <span class="slabel">{label}</span>
    <div class="smeter busy" aria-hidden="true"><i></i></div>
    <div class="sbase-r">{@render cancelBtn()}</div>
  </div>
{/snippet}

{#snippet noticeRow()}
  <div class="sbase snotice">
    <DangerCircleIcon size={16} class="nglyph" />
    <span class="nlabel">{noticeTitle}</span>
  </div>
{/snippet}

{#if o.isVisible || shownNotice}
  {#if o.state === "streaming" || liveNotice}
    <div dir="ltr" class="ov-stage {o.position}">
      {#if shownNotice}<span bind:this={probeEl} class="nlabel nprobe" aria-hidden="true">{noticeTitle}</span>{/if}
      {#key o.session}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div
          class="scard {open ? 'open' : ''} {collapsed ? 'working' : ''} {o.isVisible ? '' : 'leaving'} {noticeClass}"
          style={liveNotice && !open ? `width: ${noticeWidth}px` : undefined}
          onclick={dismissNotice}
        >
          <div class="stext">
            <div class="stext-clip">
              <div class="stext-cap {overflowing ? 'overflowing' : ''}" bind:this={capEl} onscroll={onStreamScroll}>
                <p>
                  {#if loadingNotice}<span class="sloading">Loading model…</span>{/if}
                  {#if noticeText}<span class="ndetail">{noticeText}</span>{/if}
                  <span class="committed">{o.streamText.committed ? o.streamText.committed + " " : ""}</span>
                  <span class="tentative">{o.streamText.tentative}</span>
                </p>
              </div>
            </div>
          </div>
          {#if liveNotice}
            {@render noticeRow()}
          {:else if liveWorking}
            {@render workingRow(o.workKind === "polishing" ? "Processing…" : transcribingLabel)}
          {:else}
            {@render listeningRow(open)}
          {/if}
        </div>
      {/key}
    </div>
  {:else}
    <div dir="ltr" class="ov-stage {o.position} ov-fade {o.isVisible ? 'show' : ''}">
      {#if shownNotice}<span bind:this={probeEl} class="nlabel nprobe" aria-hidden="true">{noticeTitle}</span>{/if}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div
        class="scard compact {compactWorking && o.isVisible ? 'cworking' : ''} {noticeClass}"
        style={shownNotice ? `width: ${noticeWidth}px` : undefined}
        onclick={dismissNotice}
      >
        {#if shownNotice}
          {@render noticeRow()}
        {:else if compactWorking}
          {@render workingRow(compactLabel)}
        {:else}
          {@render listeningRow(false)}
        {/if}
      </div>
    </div>
  {/if}
{/if}
