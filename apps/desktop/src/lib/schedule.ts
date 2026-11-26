export type Freq = "daily" | "weekdays" | "hourly" | "custom";

export interface Plan {
  freq: Freq;
  /** HH:MM, used by daily and weekdays. */
  time: string;
  cron: string;
}

const pad = (n: number) => String(n).padStart(2, "0");

export function toCron(freq: Freq, time: string, custom: string): string {
  const [h, m] = time.split(":").map((n) => Number.parseInt(n, 10));
  const hh = Number.isFinite(h) ? h : 0;
  const mm = Number.isFinite(m) ? m : 0;
  if (freq === "daily") return `${mm} ${hh} * * *`;
  if (freq === "weekdays") return `${mm} ${hh} * * 1-5`;
  if (freq === "hourly") return "0 * * * *";
  return custom.trim();
}

/** Maps a saved cron back to the simple form when it fits, else to custom. */
export function fromCron(cron: string): Plan {
  const c = cron.trim();
  if (c === "0 * * * *") return { freq: "hourly", time: "00:00", cron: c };
  const m = /^(\d{1,2}) (\d{1,2}) \* \* (\*|1-5)$/.exec(c);
  if (m && Number(m[1]) < 60 && Number(m[2]) < 24) {
    return { freq: m[3] === "*" ? "daily" : "weekdays", time: `${pad(Number(m[2]))}:${pad(Number(m[1]))}`, cron: c };
  }
  return { freq: "custom", time: "00:00", cron: c };
}

export function describeCron(cron: string): string {
  const p = fromCron(cron);
  if (p.freq === "daily") return `Every day at ${p.time}`;
  if (p.freq === "weekdays") return `Weekdays at ${p.time}`;
  if (p.freq === "hourly") return "Every hour";
  return `Custom: ${p.cron}`;
}
