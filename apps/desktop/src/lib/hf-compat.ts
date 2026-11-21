/**
 * Decision (product): Explore answers "will this run on my machine" before the download, with
 * tags the user can read at a glance. These are guidance, not a guarantee: the architecture list
 * is what the pinned llama.cpp build is known to run, so anything else shows as unverified.
 */
const KNOWN_ARCHITECTURES = new Set([
  "llama", "llama4", "mistral", "mistral3", "mixtral", "qwen", "qwen2", "qwen2moe", "qwen2vl", "qwen3",
  "qwen3moe", "qwen3vl", "qwen35", "gemma", "gemma2", "gemma3", "gemma3n", "gemma4", "phi2", "phi3", "phi4",
  "lfm2", "lfm3", "olmo", "olmo2", "granite", "granitemoe", "smollm3", "deepseek", "deepseek2", "command-r",
  "cohere2", "falcon", "mamba", "internlm2", "glm4", "stablelm", "starcoder2", "exaone", "minicpm", "baichuan",
]);

export type EngineSupport = "likely" | "unknown";
export type Fit = "fits" | "tight" | "too-big";

export function engineSupport(architecture: string | null | undefined): EngineSupport {
  return architecture && KNOWN_ARCHITECTURES.has(architecture.toLowerCase()) ? "likely" : "unknown";
}

const GB = 1_073_741_824;

/** Rough resident memory: the file plus working memory (KV cache and buffers). */
export const memoryNeeded = (fileBytes: number) => Math.round(fileBytes * 1.15 + 0.6 * GB);

/** Fits with room to spare, fits but would leave little, or does not fit in free memory. */
export function fitFor(fileBytes: number, freeBytes: number): Fit {
  const need = memoryNeeded(fileBytes);
  if (need <= freeBytes * 0.85) return "fits";
  if (need <= freeBytes) return "tight";
  return "too-big";
}

/** The quantization named in a GGUF file name, e.g. `Q4_K_M`, or null when there is none. */
export function quantOf(filename: string): string | null {
  const m = filename.match(/(IQ\d_[A-Z0-9]+|Q\d(?:_[A-Z0-9]+)*|MXFP4|NVFP4|BF16|F16|F32)(?=[.\-_]|$)/i);
  return m ? m[1].toUpperCase() : null;
}

/**
 * The file to preselect: the best quality that still fits comfortably, preferring the common
 * Q4_K_M, and never a file that does not fit. Falls back to the smallest file.
 */
export function suggestFile<T extends { name: string; size_bytes: number }>(files: readonly T[], freeBytes: number): T | undefined {
  const sorted = [...files].sort((a, b) => a.size_bytes - b.size_bytes);
  const fitting = sorted.filter((f) => fitFor(f.size_bytes, freeBytes) === "fits");
  return (
    fitting.find((f) => quantOf(f.name) === "Q4_K_M") ??
    fitting.at(-1) ??
    sorted.find((f) => fitFor(f.size_bytes, freeBytes) === "tight") ??
    sorted[0]
  );
}

/** 1_700_000_000 becomes "1.7B". */
export function formatParams(n: number | null | undefined): string | null {
  if (!n) return null;
  return n >= 1e9 ? `${(n / 1e9).toFixed(n >= 1e10 ? 0 : 1)}B` : `${Math.round(n / 1e6)}M`;
}

export function formatCount(n: number): string {
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(n >= 1e5 ? 0 : 1)}k`;
  return String(n);
}
