import { describe, expect, it } from "vitest";
import { engineSupport, fitFor, formatCount, formatParams, quantOf, suggestFile } from "./hf-compat";

const GB = 1_073_741_824;

describe("engineSupport", () => {
  it("knows common architectures and flags the rest as unverified", () => {
    expect(engineSupport("llama")).toBe("likely");
    expect(engineSupport("QWEN3")).toBe("likely");
    expect(engineSupport("somenewarch")).toBe("unknown");
    expect(engineSupport(null)).toBe("unknown");
  });
});

describe("fitFor", () => {
  it("rates a file against free memory", () => {
    expect(fitFor(1 * GB, 8 * GB)).toBe("fits");
    expect(fitFor(5.5 * GB, 8 * GB)).toBe("tight");
    expect(fitFor(8 * GB, 8 * GB)).toBe("too-big");
  });
});

describe("quantOf", () => {
  it("reads the quantization from common file names", () => {
    expect(quantOf("LFM3-1.7B-Q4_K_M.gguf")).toBe("Q4_K_M");
    expect(quantOf("model-q8_0.gguf")).toBe("Q8_0");
    expect(quantOf("model-IQ3_XS.gguf")).toBe("IQ3_XS");
    expect(quantOf("model-BF16.gguf")).toBe("BF16");
    expect(quantOf("plain.gguf")).toBeNull();
  });
});

describe("suggestFile", () => {
  const files = [
    { name: "m-Q2_K.gguf", size_bytes: 1 * GB },
    { name: "m-Q4_K_M.gguf", size_bytes: 2 * GB },
    { name: "m-Q8_0.gguf", size_bytes: 3 * GB },
    { name: "m-F16.gguf", size_bytes: 14 * GB },
  ];
  it("prefers Q4_K_M when it fits comfortably", () => {
    expect(suggestFile(files, 16 * GB)?.name).toBe("m-Q4_K_M.gguf");
  });
  it("never suggests a file that does not fit", () => {
    expect(suggestFile(files, 2.2 * GB)?.name).toBe("m-Q2_K.gguf");
  });
  it("falls back to the smallest file when nothing fits", () => {
    expect(suggestFile(files, 1 * GB)?.name).toBe("m-Q2_K.gguf");
  });
  it("returns nothing for no files", () => {
    expect(suggestFile([], 8 * GB)).toBeUndefined();
  });
});

describe("formatting", () => {
  it("formats parameter and download counts", () => {
    expect(formatParams(1_700_000_000)).toBe("1.7B");
    expect(formatParams(350_000_000)).toBe("350M");
    expect(formatParams(null)).toBeNull();
    expect(formatCount(1_250_000)).toBe("1.3M");
    expect(formatCount(4_200)).toBe("4.2k");
    expect(formatCount(12)).toBe("12");
  });
});
