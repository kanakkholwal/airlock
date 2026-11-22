import type { Kind, Tone } from "./Mascot.svelte";

export const KINDS: Kind[] = ["block", "star", "blob", "round", "cloud", "drop", "pill", "triangle"];
const TONES: Tone[] = ["blue", "orange", "sand"];

// A small seeded random generator, so a section always gets the same mix on every render but each
// section gets a different one.
function rng(seed: string) {
  let h = 2166136261;
  for (const c of seed) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
  return () => {
    h = Math.imul(h ^ (h >>> 15), 2246822507);
    h = Math.imul(h ^ (h >>> 13), 3266489909);
    h ^= h >>> 16;
    return (h >>> 0) / 4294967296;
  };
}

export type Pick = { kind: Kind; tone: Tone; tilt: number; delay: number; duration: number; amplitude: number };

/** `count` different characters for one section: shapes shuffled, colours mixed, motion varied. */
export function pick(section: string, count: number): Pick[] {
  const rand = rng(section);
  const kinds = [...KINDS];
  for (let i = kinds.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [kinds[i], kinds[j]] = [kinds[j], kinds[i]];
  }
  const tones = [...TONES].sort(() => rand() - 0.5);
  return kinds.slice(0, count).map((kind, i) => ({
    kind,
    // Neighbours never share a colour, so a row of them does not look stamped.
    tone: tones[(i + Math.floor(rand() * 2)) % tones.length],
    tilt: Math.round((rand() * 2 - 1) * 12),
    delay: Math.round(rand() * 2400),
    // About four to seven seconds a loop, and ten to sixteen pixels of drift.
    duration: Math.round(4000 + rand() * 3000),
    amplitude: Math.round(10 + rand() * 6),
  }));
}
