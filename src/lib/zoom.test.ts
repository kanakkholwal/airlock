import { describe, expect, it } from "vitest";
import { actualPixelsScale, clampView, containSize, zoomAt } from "./zoom";

const stage = { w: 1000, h: 800 };

describe("zoom", () => {
  it("fits a portrait image inside the stage", () => {
    expect(containSize({ w: 512, h: 768 }, stage)).toEqual({ w: (512 * 800) / 768, h: 800 });
  });

  it("keeps the picture centred at fit size", () => {
    const base = containSize({ w: 512, h: 768 }, stage);
    expect(clampView({ scale: 1, x: 90, y: -40 }, base, stage)).toEqual({ scale: 1, x: 0, y: 0 });
  });

  it("zooms around the cursor so the point under it stays put", () => {
    const base = { w: 800, h: 800 };
    const v = zoomAt({ scale: 1, x: 0, y: 0 }, 2, { x: 200, y: 0 }, base, stage);
    expect(v.scale).toBe(2);
    // The point at +200 from centre was at image offset 200; after 2x it must still sit at +200.
    expect(200 - v.x).toBeCloseTo(400);
  });

  it("never zooms past the limits", () => {
    const base = { w: 800, h: 800 };
    expect(zoomAt({ scale: 7, x: 0, y: 0 }, 10, { x: 0, y: 0 }, base, stage).scale).toBe(8);
    expect(zoomAt({ scale: 1.2, x: 0, y: 0 }, 0.1, { x: 0, y: 0 }, base, stage).scale).toBe(1);
  });

  it("offers actual pixels only when fit shows less than that", () => {
    expect(actualPixelsScale({ w: 768, h: 512 }, { w: 400, h: 267 })).toBeCloseTo(1.92);
    expect(actualPixelsScale({ w: 512, h: 512 }, { w: 800, h: 800 })).toBeNull();
  });
});
