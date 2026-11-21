<script lang="ts">
  import { tts } from "#lib/tts.svelte.js";
  import {
    VoicePicker,
    VoicePickerRow,
    VoicePickerSample,
    VoicePickerSelect,
    VoicePickerSpeed,
  } from "#lib/components/agents/voice-picker/index.js";
  import { Badge } from "#lib/components/ui/badge/index.js";

  // Decision (product): choosing a voice and hearing it are one gesture, so this is the Baby UI
  // Voice Picker fed from Airlock's voice pack, with the model it belongs to named above it.
  // Speed and voice changes apply from the next sentence of whatever is playing.
  const voices = $derived(
    (tts.status?.voices ?? []).map((v) => ({
      id: String(v.id),
      name: v.name,
      detail: `${v.accent}, ${v.gender.toLowerCase()}`,
    })),
  );
  const sample = $derived(tts.previewing ? (tts.making > 0 ? "making" : "playing") : "idle");
</script>

<div class="flex flex-col gap-3">
  <VoicePicker
    {voices}
    value={String(tts.voice)}
    onValueChange={(id) => tts.setVoice(Number(id))}
    speed={tts.speed}
    onSpeedChange={(speed) => tts.setSpeed(speed)}
    {sample}
  >
    <VoicePickerRow>
      <VoicePickerSelect />
      <VoicePickerSample onplay={() => tts.preview()} onstop={() => tts.stopPreview()} />
    </VoicePickerRow>
    <VoicePickerSpeed />
  </VoicePicker>
</div>
