import { describe, expect, it } from "vitest";
import { mentionQuery, withoutMention } from "./mention";

describe("mentions", () => {
  it("opens on a trailing @ and carries what was typed after it", () => {
    expect(mentionQuery("@")).toBe("");
    expect(mentionQuery("summarize @pod")).toBe("pod");
    expect(mentionQuery("see @notes-1")).toBe("notes-1");
  });

  it("stays closed unless the @ starts a word at the end", () => {
    expect(mentionQuery("mail me at a@b")).toBeNull();
    expect(mentionQuery("@pod and more")).toBeNull();
    expect(mentionQuery("")).toBeNull();
  });

  it("removes the open mention and keeps the sentence", () => {
    expect(withoutMention("summarize @pod")).toBe("summarize");
    expect(withoutMention("@pod")).toBe("");
    expect(withoutMention("no mention here")).toBe("no mention here");
  });
});
