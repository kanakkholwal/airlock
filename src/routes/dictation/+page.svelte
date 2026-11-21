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
  import {
    commands,
    type AutoSubmitKey,
    type ClipboardHandling,
    type OverlayPosition,
    type OverlayStyle,
    type PasteMethod,
    type ShortcutActivation,
  } from "../../bindings";
  import { inTauri } from "#lib/env.js";
  import { memory } from "#lib/memory-settings.svelte.js";
  import { speech } from "#lib/speech.svelte.js";
  import { dictation } from "#lib/settings.svelte.js";
  import DictationOrb from "#lib/components/app/dictation-orb.svelte";
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
  import { CopyButton } from "#lib/components/ui/copy-button/index.js";
  import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#lib/components/ui/collapsible/index.js";
  import { TagInput } from "#lib/components/ui/tag-input/index.js";
  import { Textarea } from "#lib/components/ui/textarea/index.js";
  import { isMac } from "#lib/dictation/access.js";

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

  const paste = isMac() ? "Cmd+V" : "Ctrl+V";
  const pasteMethods: { value: PasteMethod; label: string }[] = [
    { value: "ctrl_v", label: `Paste (${paste})` },
    { value: "direct", label: "Type it directly" },
    { value: "shift_insert", label: "Paste (Shift+Insert)" },
    { value: "ctrl_shift_v", label: "Paste (Ctrl+Shift+V)" },
    { value: "none", label: "Do not paste" },
  ];
  const clipboardOptions: { value: ClipboardHandling; label: string }[] = [
    { value: "dont_modify", label: "Put my clipboard back" },
    { value: "copy_to_clipboard", label: "Leave the text on my clipboard" },
  ];
  const positions: { value: OverlayPosition; label: string }[] = [
    { value: "bottom", label: "Bottom of the screen" },
    { value: "top", label: "Top of the screen" },
  ];
  const submitKeys: { value: AutoSubmitKey; label: string }[] = [
    { value: "enter", label: "Enter" },
    { value: "ctrl_enter", label: "Ctrl+Enter" },
    ...(isMac() ? [{ value: "cmd_enter" as AutoSubmitKey, label: "Cmd+Enter" }] : []),
  ];

  // The newest dictation (not a transcribed file), shown from History so it never depends on a
  // paste landing in this window.
  const latest = $derived(speech.history.find((h) => !speech.fileJobs.includes(h.id)));
  const latestText = $derived(latest ? latest.post_processed_text || latest.transcription_text : "");

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
          <TabsTrigger value="options">Options</TabsTrigger>
          <TabsTrigger value="files">Files</TabsTrigger>
          <TabsTrigger value="history">History</TabsTrigger>
        </TabsList>
      </Tabs>

      {#if tab === "history"}
        <HistoryList />
      {:else if tab === "files"}
        <FileTranscribe />
      {:else if tab === "options"}
        <SettingsGroup icon={MicrophoneIcon} title="While you speak" description="What you see and hear around a dictation.">
          <SettingRow title="Recording overlay" description="What shows while you speak.">
            <Select items={overlays} bind:value={() => dictation.overlayStyle, (v) => dictation.setOverlayStyle(v as OverlayStyle)}>
              <SelectTrigger class="w-56"><span class="truncate">{overlays.find((o) => o.value === dictation.overlayStyle)?.label}</span></SelectTrigger>
              <SelectContent align="end">{#each overlays as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}</SelectContent>
            </Select>
          </SettingRow>
          <SettingRow title="Overlay position" description="Where the overlay appears on your screen.">
            <Select items={positions} bind:value={() => dictation.overlayPosition, (v) => dictation.setOverlayPosition(v as OverlayPosition)}>
              <SelectTrigger class="w-56"><span class="truncate">{positions.find((o) => o.value === dictation.overlayPosition)?.label}</span></SelectTrigger>
              <SelectContent align="end">{#each positions as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}</SelectContent>
            </Select>
          </SettingRow>
          <SettingRow title="Mute other sound" description="Silences your speakers while you dictate, so music or a video does not leak into the microphone.">
            <Switch aria-label="Mute other sound while dictating" checked={dictation.muteWhileRecording} onCheckedChange={(v) => dictation.setMute(v)} />
          </SettingRow>
          <SettingRow title="Clean up with a model" description="Fixes punctuation and filler words after you speak.">
            <Switch aria-label="Clean up with a model" checked={dictation.cleanupWithLlm} onCheckedChange={(v) => dictation.setCleanup(v)} />
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup icon={KeyboardIcon} title="Text output" description="How the text gets into the app you are using.">
          <SettingRow title="How to insert text" description="Pick another method if an app ignores the default. Typing directly works where pasting is blocked.">
            <Select items={pasteMethods} bind:value={() => dictation.pasteMethod, (v) => dictation.setPasteMethod(v as PasteMethod)}>
              <SelectTrigger class="w-56"><span class="truncate">{pasteMethods.find((o) => o.value === dictation.pasteMethod)?.label}</span></SelectTrigger>
              <SelectContent align="end">{#each pasteMethods as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}</SelectContent>
            </Select>
          </SettingRow>
          <SettingRow title="Clipboard" description="Whether your own clipboard is restored after pasting.">
            <Select items={clipboardOptions} bind:value={() => dictation.clipboardHandling, (v) => dictation.setClipboardHandling(v as ClipboardHandling)}>
              <SelectTrigger class="w-64"><span class="truncate">{clipboardOptions.find((o) => o.value === dictation.clipboardHandling)?.label}</span></SelectTrigger>
              <SelectContent align="end">{#each clipboardOptions as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}</SelectContent>
            </Select>
          </SettingRow>
          <SettingRow title="Press a key after inserting" description="Sends the message for you, handy in chat boxes.">
            <Switch aria-label="Press a key after inserting" checked={dictation.autoSubmit} onCheckedChange={(v) => dictation.setAutoSubmit(v)} />
          </SettingRow>
          {#if dictation.autoSubmit}
            <SettingRow title="Which key" description="The key that is pressed after the text is inserted.">
              <Select items={submitKeys} bind:value={() => dictation.autoSubmitKey, (v) => dictation.setAutoSubmitKey(v as AutoSubmitKey)}>
              <SelectTrigger class="w-40"><span class="truncate">{submitKeys.find((o) => o.value === dictation.autoSubmitKey)?.label}</span></SelectTrigger>
              <SelectContent align="end">{#each submitKeys as o (o.value)}<SelectItem value={o.value}>{o.label}</SelectItem>{/each}</SelectContent>
            </Select>
            </SettingRow>
          {/if}
          <SettingRow title="Add a space after" description="Ends each dictation with a space, so the next one does not run into it.">
            <Switch aria-label="Add a space after each dictation" checked={dictation.appendTrailingSpace} onCheckedChange={(v) => dictation.setTrailingSpace(v)} />
          </SettingRow>
        </SettingsGroup>

        <SettingsGroup icon={SoundwaveIcon} title="Your words" description="Make dictation sound like you.">
          <SettingRow title="Remove filler words" description="Drops um, uh and similar fillers from the text.">
            <Switch aria-label="Remove filler words" checked={dictation.fillerWords} onCheckedChange={(v) => dictation.setFillerWords(v)} />
          </SettingRow>
          <div class="flex flex-col gap-2 py-3">
            <div>
              <div class="font-[450]">Custom words</div>
              <div class="text-sm text-muted-foreground">Names and terms to spell the way you want. Press Enter after each.</div>
            </div>
            <TagInput
              label="Custom words"
              placeholder="Add a word"
              bind:tags={() => dictation.customWords, (v) => void dictation.setCustomWords(v)}
            />
          </div>
        </SettingsGroup>

        <Collapsible>
          <CollapsibleTrigger class="px-1 text-sm text-muted-foreground underline underline-offset-2">Advanced</CollapsibleTrigger>
          <CollapsibleContent>
            <div class="pt-3">
              <SettingsGroup title="Advanced" description="Leave these on unless you are fixing a specific problem.">
                <SettingRow title="Skip silence" description="Detects when you are speaking, so silence is not transcribed.">
                  <Switch aria-label="Skip silence" checked={dictation.vad} onCheckedChange={(v) => dictation.setVad(v)} />
                </SettingRow>
                <SettingRow title="Split long recordings" description="Cuts long audio at pauses so it transcribes faster and more accurately.">
                  <Switch aria-label="Split long recordings" checked={dictation.longFormChunking} onCheckedChange={(v) => dictation.setLongFormChunking(v)} />
                </SettingRow>
              </SettingsGroup>
            </div>
          </CollapsibleContent>
        </Collapsible>
      {:else}
        {#if speech.needsAccess}
          <section class="flex flex-col gap-4 rounded-2xl bg-card p-5 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-300">
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
          <section class="flex flex-col gap-4 rounded-2xl bg-card p-5 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-300">
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

        <section class="flex flex-col gap-3 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 motion-safe:duration-300">
          <SectionHeader title="Try it" description="Click in the box, hold your shortcut, and say something." />
          <DictationOrb hint="Hold your shortcut and speak" />
          <Textarea placeholder="Your words appear here" rows={3} aria-label="Try dictation here" />
          {#if latestText}
            <div class="flex items-start justify-between gap-3 rounded-2xl bg-card p-4 motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-top-1 motion-safe:duration-300">
              <div class="min-w-0">
                <div class="text-sm text-muted-foreground">Latest dictation</div>
                <p class="mt-1 text-pretty whitespace-pre-wrap">{latestText}</p>
              </div>
              <CopyButton text={latestText} iconOnly label="Copy text" />
            </div>
          {/if}
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
