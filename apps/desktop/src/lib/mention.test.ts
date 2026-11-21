import { describe, expect, it } from "vitest";
import { categoriesMatching, categoryFor, mentionQuery, withBareMention, withoutMention } from "./mention";

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

describe("mention groups", () => {
  it("jumps into a group when the whole word names it", () => {
    expect(categoryFor("notes")?.id).toBe("note");
    expect(categoryFor("Recordings")?.id).toBe("recording");
    expect(categoryFor("folders")?.id).toBe("file");
    expect(categoryFor("podcast")).toBeNull();
    expect(categoryFor("")).toBeNull();
  });

  it("offers the groups a partial word could become", () => {
    expect(categoriesMatching("").length).toBe(6);
    expect(categoriesMatching("re").map((c) => c.id)).toEqual(["recording"]);
    expect(categoriesMatching("s").map((c) => c.id)).toEqual(["summary"]);
    expect(categoriesMatching("zzz")).toEqual([]);
  });

  it("keeps the list open with a bare @ after a group is chosen", () => {
    expect(withBareMention("summarize @rec")).toBe("summarize @");
    expect(withBareMention("@no")).toBe("@");
  });
});

