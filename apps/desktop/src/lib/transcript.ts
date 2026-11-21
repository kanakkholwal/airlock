export type Segment = { start_ms: number; end_ms: number; text: string };

const pad = (n: number, size = 2) => String(Math.floor(n)).padStart(size, "0");

/** `1:05` for short audio and `1:02:05` once it passes an hour. */
export function stamp(ms: number, withHours = false): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return withHours || h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

const longerThanAnHour = (segments: Segment[]) => (segments.at(-1)?.end_ms ?? 0) >= 3_600_000;

/** One line per piece, each starting with where it begins: `[0:12] Hello there.` */
export function timestamped(segments: Segment[]): string {
  const hours = longerThanAnHour(segments);
  return segments.map((s) => `[${stamp(s.start_ms, hours)}] ${s.text}`).join("\n");
}

function clock(ms: number, comma: boolean): string {
  const t = Math.max(0, Math.round(ms));
  const h = Math.floor(t / 3_600_000);
  const m = Math.floor((t % 3_600_000) / 60_000);
  const s = Math.floor((t % 60_000) / 1000);
  const milli = t % 1000;
  return `${pad(h)}:${pad(m)}:${pad(s)}${comma ? "," : "."}${pad(milli, 3)}`;
}

/** SubRip captions, which video editors and players read. */
export function toSrt(segments: Segment[]): string {
  return segments
    .map((s, i) => `${i + 1}\n${clock(s.start_ms, true)} --> ${clock(Math.max(s.end_ms, s.start_ms + 1), true)}\n${s.text}\n`)
    .join("\n");
}

/** WebVTT captions, which browsers and most players read. */
export function toVtt(segments: Segment[]): string {
  const body = segments
    .map((s) => `${clock(s.start_ms, false)} --> ${clock(Math.max(s.end_ms, s.start_ms + 1), false)}\n${s.text}\n`)
    .join("\n");
  return `WEBVTT\n\n${body}`;
}
