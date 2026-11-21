<script lang="ts">
  import { CheckCircleIcon, DangerTriangleIcon, Rocket2Icon } from "@solar-icons/svelte/linear";
  import { llm, type SetupStep } from "#lib/llm.svelte.js";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";
  import { Spinner } from "#lib/components/ui/spinner/index.js";

  // Decision (product): onboarding builds, it does not explain. One button fetches the
  // recommended engine and model and loads them, then you are in a working chat. Choosing
  // yourself stays one click away on Models. Every step is skippable by being already done.
  const plan = $derived(llm.setupPlan);
  const running = $derived(llm.setup !== null && llm.setup.step !== "failed");

  const mb = (b: number) => `${Math.round(b / 1_048_576)} MB`;
  const gb = (b: number) => `${(b / 1_073_741_824).toFixed(1)} GB`;
  const size = (b: number) => (b >= 1_073_741_824 ? gb(b) : mb(b));

  const order: SetupStep[] = ["engine", "model", "loading", "done"];
  const steps = $derived(
    plan
      ? [
          { key: "engine" as const, label: `Engine (${mb(plan.engine.size_bytes)})`, download: llm.downloads[`engine:${plan.engine.backend}`] },
          { key: "model" as const, label: `${plan.model.name} (${size(plan.model.size_bytes)})`, download: llm.downloads[plan.model.id] },
          { key: "loading" as const, label: "Load into memory", download: undefined },
        ]
      : [],
  );

  function stateOf(key: SetupStep) {
    const current = llm.setup?.step;
    if (!current || current === "failed") return "pending";
    if (current === "done" || order.indexOf(current) > order.indexOf(key)) return "done";
    return current === key ? "active" : "pending";
  }
</script>

{#if plan}
  <section class="flex flex-col gap-4 rounded-2xl bg-card p-5">
    <header class="flex items-start gap-3">
      <span class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
        <Rocket2Icon size={20} />
      </span>
      <div class="min-w-0">
        <h2 class="text-base">Get Airlock ready</h2>
        <p class="mt-0.5 text-sm text-muted-foreground text-pretty">
          One download sets up chat. After that it works offline.
        </p>
      </div>
    </header>

    {#if !running}
      <div class="flex flex-wrap items-center gap-2">
        <Button onclick={() => llm.runSetup()}>
          Set up for me ({size(plan.engine.size_bytes + plan.model.size_bytes)})
        </Button>
        <Button variant="ghost" href="/models">Choose myself</Button>
      </div>
      {#if llm.setup?.step === "failed"}
        <Alert variant="destructive">
          <DangerTriangleIcon size={18} />
          <AlertDescription>
            {llm.setup.message} Your progress is kept, so trying again continues where it stopped.
          </AlertDescription>
        </Alert>
      {/if}
    {:else}
      <ol class="flex flex-col gap-3" aria-live="polite">
        {#each steps as step (step.key)}
          {@const state = stateOf(step.key)}
          <li class="flex flex-col gap-2">
            <div class="flex items-center gap-2.5 text-sm {state === 'pending' ? 'text-muted-foreground' : ''}">
              {#if state === "done"}
                <CheckCircleIcon size={18} class="text-success-strong" />
              {:else if state === "active"}
                <Spinner size="sm" label={step.label} />
              {:else}
                <span class="size-[18px] rounded-full bg-foreground/10"></span>
              {/if}
              <span class={state === "active" ? "font-medium" : ""}>{step.label}</span>
            </div>
            {#if state === "active" && step.download}
              <Progress
                value={step.download.total > 0 ? (step.download.downloaded / step.download.total) * 100 : 0}
                indeterminate={step.download.total === 0 || step.download.phase === "verifying"}
                helper={step.download.phase === "verifying"
                  ? "Verifying"
                  : `${mb(step.download.downloaded)} of ${mb(step.download.total)} · ${(step.download.bytesPerSecond / 1_048_576).toFixed(1)} MB/s`}
              />
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
  </section>
{/if}
