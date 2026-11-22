<script lang="ts">
  import { onMount } from "svelte";
  import { Composer, ComposerInput, ComposerSubmit, ComposerToolbar } from "#lib/components/agents/composer/index.js";
  import { Message, MessageBubble, MessageContent, MessageTyping } from "#lib/components/agents/message/index.js";
  import { StreamingText, StreamingTextContent } from "#lib/components/agents/streaming-text/index.js";

  // The chat from the app, running a short script on sample text, the way a messages app plays it:
  // the question is typed in the box, it is sent and its bubble pops up, three dots show, and the
  // answer streams in. Then it rests and starts again. It is a preview, not a recording.
  const question = "Summarize my meeting notes";
  const answer = "Three points: ship the beta on Friday, Priya owns the installer, and the next sync is Tuesday at 10:00.";

  type Stage = "typing" | "sent" | "thinking" | "replying";
  let stage = $state<Stage>("typing");
  let typed = $state("");
  let round = $state(0);

  onMount(() => {
    let alive = true;
    const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    (async () => {
      while (alive) {
        stage = "typing";
        typed = "";
        for (const ch of question) {
          if (!alive) return;
          typed += ch;
          await sleep(reduced ? 0 : 55);
        }
        await sleep(450);
        stage = "sent";
        typed = "";
        await sleep(700);
        stage = "thinking";
        await sleep(1400);
        stage = "replying";
        round += 1;
        await sleep(6500);
      }
    })();
    return () => {
      alive = false;
    };
  });

  const sentOut = $derived(stage !== "typing");
</script>

<div class="flex h-full flex-col justify-between gap-4">
  <div class="flex min-h-48 flex-col justify-end gap-3">
    {#if sentOut}
      <div class="motion-safe:animate-in motion-safe:zoom-in-90 motion-safe:slide-in-from-bottom-6 motion-safe:duration-300 motion-safe:origin-bottom-right">
        <Message align="end" aria-label="You" animated={false}>
          <MessageContent><MessageBubble variant="primary">{question}</MessageBubble></MessageContent>
        </Message>
      </div>
    {/if}
    {#if stage === "thinking"}
      <div class="motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2 motion-safe:duration-200">
        <Message align="start" aria-label="Assistant" animated={false}>
          <MessageContent><MessageBubble variant="ghost"><MessageTyping /></MessageBubble></MessageContent>
        </Message>
      </div>
    {:else if stage === "replying"}
      {#key round}
        <Message align="start" aria-label="Assistant" animated={false}>
          <MessageContent>
            <MessageBubble variant="ghost">
              <StreamingText layout="inline"><StreamingTextContent content={answer} wordDelay={55} /></StreamingText>
            </MessageBubble>
          </MessageContent>
        </Message>
      {/key}
    {/if}
  </div>

  <!-- The box is part of the picture, so it is not a working field. -->
  <div inert>
    <Composer value={typed} variant="filled" onSubmit={() => {}}>
      <ComposerInput placeholder="Ask anything" minRows={1} />
      <ComposerToolbar>
        <span class="ms-auto"><ComposerSubmit /></span>
      </ComposerToolbar>
    </Composer>
  </div>
</div>
