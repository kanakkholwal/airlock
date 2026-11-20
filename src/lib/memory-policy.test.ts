import { describe, expect, it } from "vitest";
import { defaultAccelerator } from "./memory-policy";

describe("defaultAccelerator", () => {
  it("uses the CPU when every GPU is small", () => {
    expect(defaultAccelerator([{ total_vram_mb: 3962 }])).toBe("cpu");
    expect(defaultAccelerator([{ total_vram_mb: 4096 }, { total_vram_mb: 2048 }])).toBe("cpu");
  });
  it("leaves it automatic when a GPU has room", () => {
    expect(defaultAccelerator([{ total_vram_mb: 8192 }])).toBe("auto");
    expect(defaultAccelerator([{ total_vram_mb: 2048 }, { total_vram_mb: 12288 }])).toBe("auto");
  });
  it("stays automatic when there is no GPU", () => {
    expect(defaultAccelerator([])).toBe("auto");
  });
});
