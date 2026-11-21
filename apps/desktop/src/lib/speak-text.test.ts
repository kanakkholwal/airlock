import { describe, expect, it } from "vitest";
import { chunkForSpeech, plainForSpeech } from "./speak-text";

describe("plainForSpeech", () => {
  it("drops code blocks and formatting marks", () => {
    const md = "# Title\n\nUse **bold** and `code`.\n\n```js\nconst x = 1;\n```\n\n- one\n- two";
    const out = plainForSpeech(md);
    expect(out).not.toContain("const x");
    expect(out).not.toMatch(/[*#`]/);
    expect(out).toContain("Title");
    expect(out).toContain("bold");
    expect(out).toContain("one");
  });
  it("keeps link text and removes the address", () => {
    expect(plainForSpeech("See [the docs](https://example.com/x) now")).toBe("See the docs now");
    expect(plainForSpeech("Visit https://example.com today")).toBe("Visit today");
  });
  it("returns an empty string for nothing speakable", () => {
    expect(plainForSpeech("```\ncode only\n```")).toBe("");
  });
});

describe("chunkForSpeech", () => {
  it("keeps short text whole and splits long text at sentence ends", () => {
    expect(chunkForSpeech("Hello there.")).toEqual(["Hello there."]);
    const long = "One sentence here. ".repeat(30);
    const chunks = chunkForSpeech(long, 100);
    expect(chunks.length).toBeGreaterThan(3);
    expect(chunks.every((c) => c.length <= 120)).toBe(true);
    expect(chunks.join(" ").replace(/\s+/g, " ")).toBe(long.trim());
  });
});
