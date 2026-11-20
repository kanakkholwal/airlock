export type SpeedRating = "excellent" | "decent" | "usable" | "slow";

/** Decision (product): thresholds are tokens per second; 100+ is excellent, under 20 is slow. */
export function rateSpeed(tokensPerSecond: number): SpeedRating {
  if (tokensPerSecond >= 100) return "excellent";
  if (tokensPerSecond >= 50) return "decent";
  if (tokensPerSecond >= 20) return "usable";
  return "slow";
}

/** Decision (product): dictation is rated by audio seconds transcribed per second (parakeet measured 41x on Vulkan, 11x on CPU). */
export function rateRealtime(multiple: number): SpeedRating {
  if (multiple >= 20) return "excellent";
  if (multiple >= 8) return "decent";
  if (multiple >= 3) return "usable";
  return "slow";
}

export const speedLabel: Record<SpeedRating, string> = {
  excellent: "Excellent",
  decent: "Decent",
  usable: "Usable",
  slow: "Slow",
};

export type SpeedTarget = "chat" | "dictation" | "both";

export type SpeedResult = {
  target: string;
  backend: string;
  /** Tokens per second for chat; real-time multiple for dictation (audio seconds per second). */
  value: number;
  unit: string;
};
