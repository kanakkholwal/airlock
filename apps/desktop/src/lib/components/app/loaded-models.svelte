<script lang="ts">
  import { SpeedometerLowIcon } from "@solar-icons/svelte/linear";
  import { commands, type SystemMemory } from "../../../bindings";
  import { llm } from "#lib/llm.svelte.js";
  import { images } from "#lib/images.svelte.js";
  import { speaker } from "#lib/speak.svelte.js";
  import { speech } from "#lib/speech.svelte.js";
  import { rateRealtime, rateSpeed, speedLabel, type SpeedRating } from "#lib/speed.js";
  import { tts } from "#lib/tts.svelte.js";
  import {
    LoadedModels,
    LoadedModelsContent,
    LoadedModelsEmpty,
    LoadedModelsHeader,
    LoadedModelsList,
    LoadedModelsMemory,
    LoadedModelsTrigger,
    type LoadedModel,
  } from "#lib/components/ui/loaded-models/index.js";
  import { Button } from "#lib/components/ui/button/index.js";

  // Decision (product): the in-memory popover is the Baby UI Loaded Models component, fed from
  // Airlock's four kinds of model. It answers what is using memory and how to free it. The speed
  // test stays here because it needs the chat engine, which the component does not know about.
  const tone: Record<SpeedRating, "success" | "info" | "warning" | "destructive"> = {
    excellent: "success",
    decent: "info",
    usable: "warning",
    slow: "destructive",
  };
  const MB = 1_048_576;

  let open = $state(false);
  let memory = $state<SystemMemory | null>(null);

  $effect(() => {
    if (open) void commands.llmSystemMemory().then((m) => (memory = m));
  });

  const chatState = $derived(llm.engine.state);
  const chatLoaded = $derived((chatState === "ready" || chatState === "loading") && llm.current !== null);
  const chatSpeed = $derived(llm.speed && llm.speed.model === llm.current?.id ? llm.speed.value : null);

  const speechInfo = $derived(speech.models.find((m) => m.id === (speech.loadedModel ?? speech.current)));
  const dictationSpeed = $derived(
    speech.lastRun && speech.lastRun.waitSeconds > 0 ? speech.lastRun.audioSeconds / speech.lastRun.waitSeconds : null,
  );

  const voiceBusy = $derived(speaker.speakingId !== null || tts.making > 0);
  const voiceStatus = $derived(
    speaker.preparing || tts.making > 0
      ? "Preparing"
      : speaker.paused
        ? "Paused"
        : speaker.speakingId !== null
          ? `Reading${speaker.position.of > 1 ? ` ${speaker.position.at}/${speaker.position.of}` : ""}`
          : "Idle",
  );

  const models = $derived.by<LoadedModel[]>(() => {
    const list: LoadedModel[] = [];

    if (chatLoaded && llm.current) {
      const rating = chatSpeed === null ? null : rateSpeed(chatSpeed);
      list.push({
        id: "chat",
        name: llm.current.name,
        kind: "Chat",
        state: chatState === "loading" ? "loading" : "ready",
        size: llm.current.size_bytes,
        measurement:
          chatSpeed !== null && rating
            ? { value: Math.min(100, (chatSpeed / 150) * 100), tone: tone[rating], label: `${chatSpeed.toFixed(0)} tok/s`, badge: speedLabel[rating] }
            : undefined,
      });
    } else if (chatState !== "failed" && llm.engines.some((e) => e.installed) && llm.downloaded.length > 0) {
      list.push({ id: "chat", name: "Chat model", kind: "Chat", state: "idle", status: "Loads on send" });
    }

    if (speech.loadedModel || speech.loadingModel) {
      const rating = dictationSpeed === null ? null : rateRealtime(dictationSpeed);
      list.push({
        id: "speech",
        name: speechInfo?.name ?? "Speech model",
        kind: "Dictation",
        state: speech.loadingModel ? "loading" : "ready",
        size: speechInfo ? speechInfo.size_mb * MB : undefined,
        measurement:
          dictationSpeed !== null && rating
            ? { value: Math.min(100, (dictationSpeed / 40) * 100), tone: tone[rating], label: `${dictationSpeed.toFixed(0)}× realtime`, badge: speedLabel[rating] }
            : undefined,
      });
    } else if (speech.current && speechInfo) {
      list.push({ id: "speech", name: speechInfo.name, kind: "Dictation", state: "idle", status: "Loads on use" });
    }

    if (tts.ready) {
      list.push({
        id: "voice",
        name: `${tts.modelName} · ${tts.voiceName}`,
        kind: "Voice",
        state: voiceBusy ? "ready" : "idle",
        status: voiceStatus,
      });
    }

    if (images.run) {
      list.push({
        id: "image",
        name: images.model?.name ?? "Image model",
        kind: "Images",
        state: images.run.phase === "making" ? "ready" : "loading",
        status:
          images.run.phase === "queued"
            ? "Queued"
            : images.run.phase === "starting"
              ? "Starting"
              : `Step ${images.run.step}/${images.run.total}`,
      });
    }
    return list;
  });

  // Each kind frees itself its own way. For voice and images, unloading means stopping the work.
  function unload(id: string) {
    if (id === "chat") void llm.stop();
    else if (id === "speech") void speech.unload();
    else if (id === "voice") speaker.stop();
    else if (id === "image") void images.cancel();
  }

  function load(id: string) {
    if (id === "chat") void llm.ensureLoaded();
    else if (id === "speech" && speech.current) void speech.use(speech.current);
  }
</script>

<LoadedModels {models} bind:open onLoad={load} onUnload={unload}>
  <LoadedModelsTrigger />
  <LoadedModelsContent class="w-[min(26rem,calc(100vw-2rem))]">
    <LoadedModelsHeader />
    <LoadedModelsList />
    {#if memory}<LoadedModelsMemory total={memory.total_bytes} />{/if}
    <LoadedModelsEmpty />
    {#if chatState === "ready"}
      <div class="flex justify-end px-1 pt-1">
        <Button size="sm" variant="ghost" class="gap-1.5" disabled={llm.testingSpeed} onclick={() => llm.testSpeed()}>
          <SpeedometerLowIcon size={16} />{llm.testingSpeed ? "Testing" : "Test speed"}
        </Button>
      </div>
    {/if}
  </LoadedModelsContent>
</LoadedModels>
