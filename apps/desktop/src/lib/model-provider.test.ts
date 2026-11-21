import { describe, expect, it } from "vitest";
import { groupByProvider, providerOf } from "./model-provider";

describe("model providers", () => {
  it("names the maker from the id or name", () => {
    expect(providerOf({ id: "lfm3-1.7b-q4", name: "LFM3 1.7B" })).toBe("Liquid AI");
    expect(providerOf({ id: "qwen3.5-4b-abliterated-q4", name: "Qwen3.5 4B" })).toBe("Qwen");
    expect(providerOf({ id: "x", name: "Gemma 3" })).toBe("Google");
    expect(providerOf({ id: "custom-1", name: "My model" })).toBe("Other");
  });

  it("groups in first-seen order with Other last", () => {
    const g = groupByProvider([
      { provider: "Other", n: 1 },
      { provider: "Qwen", n: 2 },
      { provider: "Liquid AI", n: 3 },
      { provider: "Qwen", n: 4 },
    ]);
    expect(g.map((x) => x.provider)).toEqual(["Qwen", "Liquid AI", "Other"]);
    expect(g[0].items.map((i) => i.n)).toEqual([2, 4]);
  });
});
