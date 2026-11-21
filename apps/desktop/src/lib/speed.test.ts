import { describe, expect, it } from "vitest";
import { rateRealtime, rateSpeed } from "./speed";

describe("rateSpeed", () => {
  it("uses the product thresholds at the boundaries", () => {
    expect(rateSpeed(100)).toBe("excellent");
    expect(rateSpeed(99.9)).toBe("decent");
    expect(rateSpeed(50)).toBe("decent");
    expect(rateSpeed(49.9)).toBe("usable");
    expect(rateSpeed(20)).toBe("usable");
    expect(rateSpeed(19.9)).toBe("slow");
  });
});

describe("rateRealtime", () => {
  it("rates dictation by how many times faster than real time it is", () => {
    expect(rateRealtime(41)).toBe("excellent");
    expect(rateRealtime(11)).toBe("decent");
    expect(rateRealtime(3)).toBe("usable");
    expect(rateRealtime(1)).toBe("slow");
  });
});
