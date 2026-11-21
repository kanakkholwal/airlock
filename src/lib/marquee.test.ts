import { describe, expect, it } from "vitest";
import { marqueeSeconds } from "./marquee";

describe("marqueeSeconds", () => {
  it("does nothing when the text fits", () => {
    expect(marqueeSeconds(0)).toBe(0);
    expect(marqueeSeconds(-5)).toBe(0);
  });
  it("keeps a steady speed within limits", () => {
    expect(marqueeSeconds(90)).toBe(2);
    expect(marqueeSeconds(10)).toBe(0.6);
    expect(marqueeSeconds(5000)).toBe(8);
  });
});
