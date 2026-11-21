import { describe, expect, it } from "vitest";
import { formatBytes, formatEta, formatSpeed } from "./format";

describe("format", () => {
  it("picks the unit by size", () => {
    expect(formatBytes(2_500_000_000)).toBe("2.3 GB");
    expect(formatBytes(335_304_388)).toBe("320 MB");
    expect(formatBytes(2048)).toBe("2 KB");
  });
  it("formats speed", () => {
    expect(formatSpeed(10 * 1_048_576)).toBe("10 MB/s");
  });
  it("hides the estimate until speed is known", () => {
    expect(formatEta(1_000_000_000, 0)).toBeNull();
    expect(formatEta(600 * 1_048_576, 1_048_576)).toBe("about 10 min left");
    expect(formatEta(10 * 1_048_576, 1_048_576)).toBe("under a minute left");
  });
});
