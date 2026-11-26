import { describe, expect, it } from "vitest";
import { describeCron, fromCron, toCron } from "./schedule";

describe("schedule", () => {
  it("round-trips the simple forms", () => {
    expect(toCron("daily", "00:00", "")).toBe("0 0 * * *");
    expect(toCron("weekdays", "18:30", "")).toBe("30 18 * * 1-5");
    expect(fromCron("30 18 * * 1-5")).toMatchObject({ freq: "weekdays", time: "18:30" });
    expect(fromCron("0 0 * * *")).toMatchObject({ freq: "daily", time: "00:00" });
  });

  it("falls back to custom for anything else", () => {
    expect(fromCron("*/15 * * * *").freq).toBe("custom");
    expect(fromCron("0 99 * * *").freq).toBe("custom");
    expect(describeCron("*/15 * * * *")).toBe("Custom: */15 * * * *");
  });

  it("describes in plain words", () => {
    expect(describeCron("0 0 * * *")).toBe("Every day at 00:00");
    expect(describeCron("0 * * * *")).toBe("Every hour");
  });
});
