<script lang="ts">
  import { onMount } from "svelte";
  import {
    AddCircleIcon,
    ClockCircleIcon,
    DangerTriangleIcon,
    PenIcon,
    PlayIcon,
    TrashBinMinimalisticIcon,
  } from "@solar-icons/svelte/linear";
  import type { Automation } from "../../bindings";
  import { ACTIONS, actionLabel, automations } from "#lib/automations.svelte.js";
  import { inTauri } from "#lib/env.js";
  import { formatDate } from "#lib/format.js";
  import { describeCron, fromCron, toCron, type Freq } from "#lib/schedule.js";
  import CardSkeleton from "#lib/components/app/card-skeleton.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import StateView from "#lib/components/app/state-view.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Dialog, DialogContent, DialogDescription, DialogTitle } from "#lib/components/ui/dialog/index.js";
  import { Input } from "#lib/components/ui/input/index.js";
  import { Label } from "#lib/components/ui/label/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";

  // Decision (product): automations run on this computer, in the user's own time zone, with local
  // models only, so scheduled work never spends tokens. They run while Airlock is open; a run
  // missed while it was closed is skipped or run once, per automation.
  const FREQS: { value: Freq; label: string }[] = [
    { value: "daily", label: "Every day" },
    { value: "weekdays", label: "Weekdays" },
    { value: "hourly", label: "Every hour" },
    { value: "custom", label: "Custom (cron)" },
  ];
  const CATCHUP = [
    { value: "run_once", label: "Run once when Airlock opens" },
    { value: "skip", label: "Skip it" },
  ];
  const PRESETS = [
    { name: "Summarize new notes every night", freq: "daily" as Freq, time: "00:00" },
    { name: "Summarize new notes on weekday evenings", freq: "weekdays" as Freq, time: "18:00" },
  ];

  const zones = $derived([
    { value: "system", label: `This computer (${automations.zone?.system ?? "..."})` },
    ...(typeof Intl !== "undefined" && "supportedValuesOf" in Intl ? (Intl as any).supportedValuesOf("timeZone") : []).map((z: string) => ({ value: z, label: z })),
  ]);

  let editOpen = $state(false);
  let editing = $state<number | null>(null);
  let name = $state("");
  let action = $state(ACTIONS[0].value);
  let freq = $state<Freq>("daily");
  let time = $state("00:00");
  let custom = $state("");
  let catchup = $state("run_once");
  let formError = $state<string | null>(null);
  let saving = $state(false);
  let deleting = $state<Automation | null>(null);
  let deleteOpen = $state(false);
  let zoneError = $state<string | null>(null);

  onMount(() => void automations.init());

  function openNew(preset?: (typeof PRESETS)[number]) {
    editing = null;
    name = preset?.name ?? "";
    action = ACTIONS[0].value;
    freq = preset?.freq ?? "daily";
    time = preset?.time ?? "00:00";
    custom = "";
    catchup = "run_once";
    formError = null;
    editOpen = true;
  }

  function openEdit(a: Automation) {
    const p = fromCron(a.cron);
    editing = a.id;
    name = a.name;
    action = a.action;
    freq = p.freq;
    time = p.time;
    custom = p.cron;
    catchup = a.catchup;
    formError = null;
    editOpen = true;
  }

  async function submit() {
    saving = true;
    const existing = automations.items.find((a) => a.id === editing);
    formError = await automations.save({
      id: editing,
      name,
      cron: toCron(freq, time, custom),
      action,
      enabled: existing?.enabled ?? true,
      catchup,
    });
    saving = false;
    if (!formError) editOpen = false;
  }

  function askDelete(a: Automation) {
    deleting = a;
    deleteOpen = true;
  }

  async function confirmDelete() {
    if (deleting) await automations.remove(deleting.id);
    deleteOpen = false;
  }

  async function changeZone(value: string) {
    zoneError = await automations.setZone(value);
  }

  const statusText = (a: Automation) => {
    const r = automations.lastRun(a.id);
    if (automations.running.includes(a.id) || r?.status === "running") return "Running now";
    if (!r) return "Has not run yet";
    return `${r.status === "failed" ? "Failed" : r.status === "skipped" ? "Skipped" : "Last run"} ${formatDate(r.started_ms)}${r.message ? `: ${r.message}` : ""}`;
  };
</script>

<PageScroll>
  <div class="mx-auto flex w-full max-w-3xl flex-col gap-6 px-6 py-12">
    <PageHeader title="Automations" description="Run things on a schedule, on this computer. Local models only, so nothing is spent and nothing leaves." />

    {#if !inTauri()}
      <StateView icon={ClockCircleIcon} title="Open the Airlock app to set up automations" />
    {:else if !automations.loaded}
      <CardSkeleton count={2} />
    {:else}
      <div class="flex flex-wrap items-center justify-between gap-3 rounded-2xl bg-card p-4">
        <div class="min-w-0">
          <p class="text-sm font-medium">Time zone</p>
          <p class="text-sm text-muted-foreground">Schedules follow this zone. Now: {automations.zone?.effective ?? "..."}</p>
        </div>
        <Select items={zones} value={automations.zone?.setting ?? "system"} onValueChange={(v) => void changeZone(v)}>
          <SelectTrigger class="w-64"><span class="truncate">{zones.find((z) => z.value === (automations.zone?.setting ?? "system"))?.label}</span></SelectTrigger>
          <SelectContent align="end">
            {#each zones as z (z.value)}<SelectItem value={z.value}>{z.label}</SelectItem>{/each}
          </SelectContent>
        </Select>
      </div>

      {#if zoneError || automations.error}
        <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{zoneError ?? automations.error}</AlertDescription></Alert>
      {/if}

      {#if automations.items.length === 0}
        <StateView
          icon={ClockCircleIcon}
          title="Nothing scheduled yet"
          description="Pick one to start, or make your own. Runs only while Airlock is open."
        >
          {#snippet actions()}
            <div class="flex flex-wrap justify-center gap-2">
              {#each PRESETS as p (p.name)}
                <Button variant="secondary" class="gap-1.5" onclick={() => openNew(p)}><AddCircleIcon size={16} />{p.name}</Button>
              {/each}
              <Button variant="ghost" onclick={() => openNew()}>Custom</Button>
            </div>
          {/snippet}
        </StateView>
      {:else}
        <ul class="flex flex-col gap-3">
          {#each automations.items as a (a.id)}
            <li class="flex flex-col gap-3 rounded-2xl bg-card p-4 motion-safe:animate-in motion-safe:fade-in motion-safe:duration-300">
              <div class="flex items-start gap-3">
                <span class="flex size-9 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground"><ClockCircleIcon size={18} /></span>
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm font-medium">{a.name}</p>
                  <p class="text-sm text-muted-foreground">{actionLabel(a.action)} · {describeCron(a.cron)}</p>
                  <p class="mt-1 text-xs text-muted-foreground tabular-nums">
                    {a.enabled && a.next_run_ms ? `Next: ${formatDate(a.next_run_ms)}` : "Paused"}
                  </p>
                  <p class="text-xs tabular-nums {automations.lastRun(a.id)?.status === 'failed' ? 'text-destructive-strong' : 'text-muted-foreground'}">{statusText(a)}</p>
                </div>
                <Switch aria-label="Run {a.name} on schedule" checked={a.enabled} onCheckedChange={(v) => automations.setEnabled(a, v)} />
              </div>
              <div class="flex flex-wrap items-center gap-2 pl-12">
                <Button size="sm" variant="secondary" class="gap-1.5" disabled={automations.running.includes(a.id)} onclick={() => automations.runNow(a.id)}>
                  <PlayIcon size={16} />Run now
                </Button>
                <Button size="sm" variant="ghost" class="gap-1.5" onclick={() => openEdit(a)}><PenIcon size={16} />Edit</Button>
                <Button size="sm" variant="ghost" class="gap-1.5 text-muted-foreground" onclick={() => askDelete(a)}><TrashBinMinimalisticIcon size={16} />Delete</Button>
                {#if a.catchup === "skip"}<Badge size="sm" variant="secondary">Skips missed runs</Badge>{/if}
              </div>
            </li>
          {/each}
        </ul>
        <div class="flex justify-center">
          <Button variant="secondary" class="gap-1.5" onclick={() => openNew()}><AddCircleIcon size={16} />New automation</Button>
        </div>
      {/if}
    {/if}
  </div>
</PageScroll>

<Dialog bind:open={editOpen} size="md">
  <DialogContent>
    <DialogTitle>{editing === null ? "New automation" : "Edit automation"}</DialogTitle>
    <DialogDescription>Runs on this computer, in the time zone above.</DialogDescription>
    <form
      class="mt-4 flex flex-col gap-4"
      onsubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <div class="flex flex-col gap-1.5">
        <Label for="auto-name">Name</Label>
        <Input id="auto-name" bind:value={name} placeholder="Summarize new notes every night" />
      </div>
      <div class="flex flex-col gap-1.5">
        <Label>What it does</Label>
        <Select items={ACTIONS} bind:value={action}>
          <SelectTrigger class="w-full"><span class="truncate">{actionLabel(action)}</span></SelectTrigger>
          <SelectContent>
            {#each ACTIONS as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
          </SelectContent>
        </Select>
        <p class="text-xs text-muted-foreground">{ACTIONS.find((o) => o.value === action)?.hint}</p>
      </div>
      <div class="flex flex-wrap gap-3">
        <div class="flex flex-1 flex-col gap-1.5">
          <Label>When</Label>
          <Select items={FREQS} bind:value={freq}>
            <SelectTrigger class="w-full"><span class="truncate">{FREQS.find((o) => o.value === freq)?.label}</span></SelectTrigger>
            <SelectContent>
              {#each FREQS as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
            </SelectContent>
          </Select>
        </div>
        {#if freq === "daily" || freq === "weekdays"}
          <div class="flex flex-col gap-1.5">
            <Label for="auto-time">At</Label>
            <Input id="auto-time" type="time" bind:value={time} class="w-32" />
          </div>
        {/if}
      </div>
      {#if freq === "custom"}
        <div class="flex flex-col gap-1.5">
          <Label for="auto-cron">Cron expression</Label>
          <Input id="auto-cron" bind:value={custom} placeholder="*/30 * * * *" class="font-mono" />
        </div>
      {/if}
      <div class="flex flex-col gap-1.5">
        <Label>If Airlock was closed at that time</Label>
        <Select items={CATCHUP} bind:value={catchup}>
          <SelectTrigger class="w-full"><span class="truncate">{CATCHUP.find((o) => o.value === catchup)?.label}</span></SelectTrigger>
          <SelectContent>
            {#each CATCHUP as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}
          </SelectContent>
        </Select>
      </div>
      {#if formError}
        <Alert variant="destructive"><DangerTriangleIcon size={18} /><AlertDescription>{formError}</AlertDescription></Alert>
      {/if}
      <div class="flex justify-end gap-2">
        <Button type="button" variant="ghost" onclick={() => (editOpen = false)}>Cancel</Button>
        <Button type="submit" disabled={saving}>{editing === null ? "Create" : "Save"}</Button>
      </div>
    </form>
  </DialogContent>
</Dialog>

<Dialog bind:open={deleteOpen} size="sm">
  <DialogContent>
    <DialogTitle>Delete this automation?</DialogTitle>
    <DialogDescription>{deleting?.name}. Its run history goes too.</DialogDescription>
    <div class="mt-4 flex justify-end gap-2">
      <Button variant="ghost" onclick={() => (deleteOpen = false)}>Cancel</Button>
      <Button variant="destructive" onclick={confirmDelete}>Delete</Button>
    </div>
  </DialogContent>
</Dialog>
