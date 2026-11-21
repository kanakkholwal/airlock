export type Backend = "metal" | "cuda" | "vulkan" | "cpu";
export type Gpu = { vendor: "nvidia" | "amd" | "intel" | "apple" | "other"; vramMiB: number };
export type Probe = {
  os: "windows" | "macos" | "linux";
  gpus: Gpu[];
  vulkan: boolean;
  cudaInstalled: boolean;
};

/**
 * Decision (architecture): Vulkan is the default on Windows/Linux, Metal on macOS, CPU last.
 * CUDA is an opt-in download for NVIDIA, since it measured only ~10-16% faster than Vulkan.
 */
export function chooseBackend(p: Probe): Backend {
  if (p.os === "macos") return "metal";
  if (p.cudaInstalled && p.gpus.some((g) => g.vendor === "nvidia")) return "cuda";
  if (p.vulkan && p.gpus.length > 0) return "vulkan";
  return "cpu";
}

export function offerCuda(p: Probe): boolean {
  return p.os !== "macos" && !p.cudaInstalled && p.gpus.some((g) => g.vendor === "nvidia");
}
