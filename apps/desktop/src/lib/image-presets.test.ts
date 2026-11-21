import { describe, expect, it } from "vitest";
import type { ImageInfo } from "../bindings";
import { filterImages, shapeCounts, shapeOf } from "./image-presets";

const img = (id: string, w: number, h: number, prompt: string, created_ms: number): ImageInfo =>
  ({ id, path: `/${id}.png`, prompt, model: "m", width: w, height: h, steps: 8, seed: 1, created_ms }) as ImageInfo;

const all = [
  img("a", 512, 512, "A red fox", 1),
  img("b", 512, 768, "A blue lighthouse", 3),
  img("c", 768, 512, "A red barn", 2),
];

describe("image presets", () => {
  it("classifies a picture by its own size", () => {
    expect(shapeOf(all[0])).toBe("square");
    expect(shapeOf(all[1])).toBe("portrait");
    expect(shapeOf(all[2])).toBe("landscape");
  });

  it("filters by words in the prompt and by shape, then sorts", () => {
    expect(filterImages(all, { query: "red", shape: "all", sort: "newest" }).map((i) => i.id)).toEqual(["c", "a"]);
    expect(filterImages(all, { query: "", shape: "portrait", sort: "newest" }).map((i) => i.id)).toEqual(["b"]);
    expect(filterImages(all, { query: "", shape: "all", sort: "oldest" }).map((i) => i.id)).toEqual(["a", "c", "b"]);
  });

  it("does not change the list it is given", () => {
    const copy = [...all];
    filterImages(all, { query: "", shape: "all", sort: "newest" });
    expect(all).toEqual(copy);
  });

  it("counts only shapes that exist", () => {
    expect(shapeCounts([all[0], all[1]]).map((s) => s.value)).toEqual(["square", "portrait"]);
    expect(shapeCounts([])).toEqual([]);
  });
});
