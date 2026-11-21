const PIXELS_PER_SECOND = 45;

/** Seconds to slide `overflow` pixels of hidden text into view: steady speed, never jarring. */
export function marqueeSeconds(overflow: number): number {
  if (overflow <= 0) return 0;
  return Math.min(8, Math.max(0.6, overflow / PIXELS_PER_SECOND));
}
