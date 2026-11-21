/**
 * Decision (product): speech and chat share one small GPU. Speech needs almost no GPU (a short
 * dictation takes under a second on the CPU), so on a GPU with little memory it runs on the CPU
 * and chat keeps the whole GPU. The limit is a judgement: below it, the desktop, the chat model
 * and the speech model together tend to spill into shared memory, which slows everything.
 */
export const SMALL_GPU_MB = 6144;

export type Accelerator = "auto" | "cpu" | "gpu";

/** The accelerator to start with: the CPU when every GPU is small, otherwise automatic. */
export function defaultAccelerator(gpus: readonly { total_vram_mb: number }[]): Accelerator {
  if (gpus.length === 0) return "auto";
  return gpus.every((g) => g.total_vram_mb < SMALL_GPU_MB) ? "cpu" : "auto";
}
