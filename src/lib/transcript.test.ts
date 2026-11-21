import { describe, expect, it } from "vitest";
import { stamp, timestamped, toSrt, toVtt } from "./transcript";

const segs = [
  { start_ms: 0, end_ms: 1500, text: "Hello there." },
  { start_ms: 65_000, end_ms: 66_250, text: "A minute in." },
];

describe("transcript formats", () => {
  it("writes short times as m:ss and long ones with hours", () => {
    expect(stamp(5_000)).toBe("0:05");
    expect(stamp(65_000)).toBe("1:05");
    expect(stamp(3_725_000)).toBe("1:02:05");
    expect(stamp(5_000, true)).toBe("0:00:05");
  });

  it("starts each line with its time", () => {
    expect(timestamped(segs)).toBe("[0:00] Hello there.\n[1:05] A minute in.");
  });

  it("switches every line to hours once the audio is an hour long", () => {
    const long = [...segs, { start_ms: 3_600_000, end_ms: 3_601_000, text: "Late." }];
    expect(timestamped(long).split("\n")[0]).toBe("[0:00:00] Hello there.");
  });

  it("writes SubRip with commas and numbered cues", () => {
    const srt = toSrt(segs);
    expect(srt.startsWith("1\n00:00:00,000 --> 00:00:01,500\nHello there.")).toBe(true);
    expect(srt).toContain("2\n00:01:05,000 --> 00:01:06,250\nA minute in.");
  });

  it("writes WebVTT with a header and dots", () => {
    expect(toVtt(segs).startsWith("WEBVTT\n\n00:00:00.000 --> 00:00:01.500")).toBe(true);
  });

  it("never writes an empty cue", () => {
    expect(toSrt([{ start_ms: 1000, end_ms: 1000, text: "x" }])).toContain("00:00:01,000 --> 00:00:01,001");
  });
});
