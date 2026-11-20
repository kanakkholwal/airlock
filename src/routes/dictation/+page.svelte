<script lang="ts">
  import { onMount } from "svelte";
  import {
    KeyboardIcon,
    CheckCircleIcon,
    Microphone3Icon,
    MicrophoneIcon,
    ShieldCheckIcon,
    SoundwaveIcon,
  } from "@solar-icons/svelte/linear";
  import { commands, type OverlayStyle, type ShortcutActivation } from "../../bindings";
  import { inTauri } from "#lib/env.js";
  import { memory } from "#lib/memory-settings.svelte.js";
  import { speech } from "#lib/speech.svelte.js";
  import { dictation } from "#lib/settings.svelte.js";
  import FileTranscribe from "#lib/components/app/file-transcribe.svelte";
  import HistoryList from "#lib/components/app/history-list.svelte";
  import PageHeader from "#lib/components/app/page-header.svelte";
  import PageScroll from "#lib/components/app/page-scroll.svelte";
  import SectionHeader from "#lib/components/app/section-header.svelte";
  import SettingRow from "#lib/components/app/setting-row.svelte";
  import SettingsGroup from "#lib/components/app/settings-group.svelte";
  import SettingsSkeleton from "#lib/components/app/settings-skeleton.svelte";
  import { Alert, AlertDescription } from "#lib/components/ui/alert/index.js";
  import { Button } from "#lib/components/ui/button/index.js";
  import { Progress } from "#lib/components/ui/progress/index.js";
  import { Select, SelectContent, SelectItem, SelectTrigger } from "#lib/components/ui/select/index.js";
  import { ShortcutRecorder } from "#lib/components/ui/shortcut-recorder/index.js";
  import { Switch } from "#lib/components/ui/switch/index.js";
  import { Tabs, TabsList, TabsTrigger } from "#lib/components/ui/tabs/index.js";
  import { Textarea } from "#lib/components/ui/textarea/index.js";

  // Decision (product): people open this to set talking-to-text up once, check it works, and
  // later find what they said. So: setup is one button, trying it is on the page, and history
  // is a tab, not another place to look for.
  let tab = $state("setup");

  const activations: { value: ShortcutActivation; label: string }[] = [
    { value: "hold_or_toggle", label: "Hold or tap" },
    { value: "push_to_talk", label: "Hold to talk" },
    { value: "toggle", label: "Press to start and stop" },
  ];
  const overlays: { value: OverlayStyle; label: string }[] = [
    { value: "minimal", label: "Minimal pill" },
    { value: "live", label: "Live transcript" },
    { value: "none", label: "Hidden" },
  ];

  const mics = $derived([
    { value: "Default", label: "System default" },
    ...speech.mics.filter((m) => !m.is_default).map((m) => ({ value: m.name, label: m.name })),
  ]);
  const setup = $derived(speech.setupModel);
  const setupProgress = $derived(setup ? speech.progress[setup.id] : undefined);
  const mb = (b: number) => `${Math.round(b / 1_048_576)} MB`;
  const size = (m: number) => (m >= 1024 ? `${(m / 1024).toFixed(1)} GB` : `${Math.round(m)} MB`);

  onMount(() => {
    void dictation.load();
    void speech.init();
    void memory.load();
  });
</script>

<PageScroll>
  <main class="mx-auto flex min-h-full w-full max-w-3xl flex-col gap-8 px-6 py-12">
    <PageHeader title="Dictation" description="Hold a key, speak, and the text lands where you were typing." />

    {#if !inTauri()}
      <Alert variant="info">
        <AlertDescription>This is a browser preview, so dictation settings are not saved. Open the desktop app to change them.</AlertDescription>
      </Alert>
    {/if}
    {#if dictation.error || speech.error}
      <Alert variant="destructive"><AlertDescription>{dictation.error ?? speech.error}</AlertDescription></Alert>
    {/if}

    {#if inTauri() && (!dictation.loaded || !speech.loaded) && !dictation.error}
      <SettingsSkeleton groups={3} rows={2} />
    {:else}
      <Tabs bind:value={tab} variant="segment" class="self-center">
        <TabsList>
          <TabsTrigger value="setup">Setup</TabsTrigger>
          <TabsTrigger value="files">Files</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
      </Tabs>

      {#if tab === "history"}
        <HistoryList />
      {:else if tab === "files"}
        <FileTranscribe />
      {:else}
        {#if speech.needsAccess}
          <section class="flex flex-col gap-4 rounded-2xl bg-card p-5">
            <header class="flex items-start gap-3">
              <span class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-warning/15 text-warning-strong">
                <ShieldCheckIcon size={20} />
              </span>
              <div class="min-w-0">
                <h2 class="text-lg">Allow Airlock to listen and type</h2>
                <p class="mt-0.5 text-sm text-muted-foreground text-pretty">
                  macOS needs your permission for the shortcut to work in other apps and for Airlock to hear you.
                  Nothing is recorded until you press the shortcut.
                </p>
              </div>
            </header>
            <ul class="flex flex-col gap-2">
              {#each [{ key: "accessibility" as const, title: "Accessibility", detail: "Lets Airlock see your shortcut and type the text for you." }, { key: "microphone" as const, title: "Microphone", detail: "Lets Airlock hear you while you hold the shortcut." }] as p (p.key)}
                <li class="flex items-center justify-between gap-4 rounded-xl bg-muted/60 px-4 py-3">
                  <div class="min-w-0">
                    <div class="font-[450]">{p.title}</div>
                    <div class="text-sm text-muted-foreground">{p.detail}</div>
                  </div>
                  {#if speech.access[p.key]}
                    <span class="flex items-center gap-1.5 text-sm text-success-strong"><CheckCircleIcon size={18} />Allowed</span>
                  {:else}
                    <Button size="sm" onclick={() => speech.allow(p.key)}>Allow</Button>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if speech.needsSetup && setup}
          <section class="flex flex-col gap-4 rounded-2xl bg-card p-5">
            <header class="flex items-start gap-3">
              <span class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
                <MicrophoneIcon size={20} />
              </span>
              <div class="min-w-0">
                <h2 class="text-lg">Set up dictation</h2>
                <p class="mt-0.5 text-sm text-muted-foreground text-pretty">
                  Dictation needs a speech model ({setup.name}, {size(setup.size_mb)}). It downloads once, then works
                  offline in any app.
                </p>
              </div>
            </header>
            {#if speech.settingUp}
              <Progress
                value={setupProgress?.percentage ?? 0}
                indeterminate={!setupProgress || setupProgress.total === 0}
                helper={setupProgress ? `${mb(setupProgress.downloaded)} of ${mb(setupProgress.total)}` : "Starting"}
              />
            {:else}
              <div class="flex flex-wrap gap-2">
                <Button onclick={() => speech.runSetup()}>Set up for me ({size(setup.size_mb)})</Button>
              </div>
            {/if}
          </section>
        {/if}

        <SettingsGroup icon={SoundwaveIcon} title="Speech model" description="What turns your voice into text. It runs on this computer.">
          <SettingRow
            title={speech.models.find((m) => m.id === speech.current)?.name ?? "None selected"}
            description={speech.current ? "In use for dictation." : "Choose one to start dictating."}
          >
            <Button size="sm" variant="secondary" href="/models?tab=speech">{speech.current ? "Change" : "Choose"}</Button>
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup icon={Microphone3Icon} title="Microphone" description="Which microphone dictation listens to.">
          <SettingRow title="Input device" description="Use the system default unless you have a headset you prefer.">
            <Select items={mics} bind:value={() => speech.mic, (v) => void speech.setMic(v as string)}>
              <SelectTrigger class="w-64">
                <span class="truncate">{mics.find((m) => m.value === speech.mic)?.label ?? speech.mic}</span>
              </SelectTrigger>
              <SelectContent align="end">
                {#each mics as m (m.value)}
                  <SelectItem value={m.value}>{m.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </SettingRow>
        </SettingsGroup>
        {#if speech.micBlocked}
          <Alert variant="warning">
            <AlertDescription>
              Windows is blocking microphone access for Airlock, so dictation will hear nothing.
              <span class="mt-2 flex gap-2">
                <Button size="sm" variant="secondary" onclick={() => commands.openMicrophonePrivacySettings()}>Open privacy settings</Button>
                <Button size="sm" variant="ghost" onclick={() => speech.refreshPermission()}>Check again</Button>
              </span>
            </AlertDescription>
          </Alert>
        {/if}

        <SettingsGroup icon={KeyboardIcon} title="Shortcut" description="How you start and stop talking.">
          <SettingRow title="Dictation shortcut" description="Works in any app.">
            <ShortcutRecorder
              label="Dictation shortcut"
              value={dictation.shortcut}
              defaultValue="ctrl+space"
              onValueChange={(v) => dictation.setShortcut(v)}
            />
          </SettingRow>
          <SettingRow title="Activation" description="How the shortcut starts and stops recording.">
            <Select
              items={activations}
              bind:value={() => dictation.activation, (v) => dictation.setActivation(v as ShortcutActivation)}
            >
              <SelectTrigger class="w-56">
                <span class="truncate">{activations.find((a) => a.value === dictation.activation)?.label}</span>
              </SelectTrigger>
              <SelectContent align="end">
                {#each activations as a (a.value)}
                  <SelectItem value={a.value}>{a.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup icon={MicrophoneIcon} title="While you speak" description="What you see and hear around a dictation.">
          <SettingRow title="Recording overlay" description="What shows while you speak.">
            <Select
              items={overlays}
              bind:value={() => dictation.overlayStyle, (v) => dictation.setOverlayStyle(v as OverlayStyle)}
            >
              <SelectTrigger class="w-56">
                <span class="truncate">{overlays.find((o) => o.value === dictation.overlayStyle)?.label}</span>
              </SelectTrigger>
              <SelectContent align="end">
                {#each overlays as o (o.value)}
                  <SelectItem value={o.value}>{o.label}</SelectItem>
                {/each}
              </SelectContent>
            </Select>
          </SettingRow>
          <SettingRow title="Mute other sound" description="Silences your speakers while you dictate, so music or a video does not leak into the microphone.">
            <Switch
              aria-label="Mute other sound while dictating"
              checked={dictation.muteWhileRecording}
              onCheckedChange={(v) => dictation.setMute(v)}
            />
          </SettingRow>
          <SettingRow title="Clean up with a model" description="Fixes punctuation and filler words after you speak.">
            <Switch
              aria-label="Clean up with a model"
              checked={dictation.cleanupWithLlm}
              onCheckedChange={(v) => dictation.setCleanup(v)}
            />
          </SettingRow>
        </SettingsGroup>

        <section class="flex flex-col gap-3">
          <SectionHeader title="Try it" description="Click in the box, hold your shortcut, and say something." />
          <Textarea placeholder="Your words appear here" rows={3} aria-label="Try dictation here" />
          {#if speech.lastRun}
            {@const r = speech.lastRun}
            <p class="px-1 text-sm text-muted-foreground tabular-nums">
              Last dictation: {r.audioSeconds.toFixed(1)} s of speech, text ready in {r.waitSeconds.toFixed(2)} s
              ({r.waitSeconds > 0 ? (r.audioSeconds / r.waitSeconds).toFixed(1) : "many"}x faster than real time) on
              {memory.accelerator === "cpu" ? "the CPU" : memory.accelerator === "gpu" ? "the GPU" : "automatic"}.
            </p>
          {/if}
        </section>
      {/if}
    {/if}
  </main>
</PageScroll>
