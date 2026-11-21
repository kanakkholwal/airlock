import { describe, expect, it } from "vitest";
import { groupByRecency } from "./chat-groups";

const NOW = new Date(2026, 10, 20, 15, 0, 0).getTime();
const at = (daysAgo: number, hour = 12) =>
  new Date(2026, 10, 20 - daysAgo, hour, 0, 0).getTime();

describe("groupByRecency", () => {
  it("puts chats into the right buckets and drops empty ones", () => {
    const chats = [
      { id: 1, updated_at: at(0, 14) },
      { id: 2, updated_at: at(0, 1) },
      { id: 3, updated_at: at(1) },
      { id: 4, updated_at: at(5) },
      { id: 5, updated_at: at(20) },
      { id: 6, updated_at: at(90) },
    ];
    const groups = groupByRecency(chats, NOW);
    expect(groups.map((g) => g.label)).toEqual([
      "Today",
      "Yesterday",
      "Previous 7 days",
      "Previous 30 days",
      "Older",
    ]);
    expect(groups[0].items.map((c) => c.id)).toEqual([1, 2]);
  });

  it("treats just after midnight as today and just before as yesterday", () => {
    const groups = groupByRecency(
      [
        { id: 1, updated_at: new Date(2026, 10, 20, 0, 0, 1).getTime() },
        { id: 2, updated_at: new Date(2026, 10, 19, 23, 59, 59).getTime() },
      ],
      NOW,
    );
    expect(groups.map((g) => [g.label, g.items[0].id])).toEqual([
      ["Today", 1],
      ["Yesterday", 2],
    ]);
  });

  it("returns nothing for an empty list", () => {
    expect(groupByRecency([], NOW)).toEqual([]);
  });
});
