import { describe, expect, it } from "vitest";
import { pickAutoModel } from "./model-pick";

const a = { id: "a", recommended: false };
const b = { id: "b", recommended: true };
const c = { id: "c", recommended: false };

describe("pickAutoModel", () => {
  it("prefers the last used model", () => {
    expect(pickAutoModel([a, b, c], "c")).toBe(c);
  });
  it("falls back to the recommended model when the last one is gone", () => {
    expect(pickAutoModel([a, b], "deleted")).toBe(b);
    expect(pickAutoModel([a, b], null)).toBe(b);
  });
  it("falls back to the first model on disk", () => {
    expect(pickAutoModel([a, c], null)).toBe(a);
  });
  it("returns nothing when no model is downloaded", () => {
    expect(pickAutoModel([], "a")).toBeUndefined();
  });
});
