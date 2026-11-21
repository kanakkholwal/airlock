import { describe, expect, it } from "vitest";
import { cleanReadme } from "./readme-clean";

describe("cleanReadme", () => {
  it("removes html blocks, comments and images but keeps text", () => {
    const md = '<div align="center"><a href="x"><img src="y"/></a></div>\n\n# Title\n\n<!-- hidden -->\nSome **text**.\n\n![demo](https://a/b.png)';
    expect(cleanReadme(md)).toBe("# Title\n\nSome **text**.");
  });

  it("drops badge links whose only content was an image", () => {
    expect(cleanReadme("[![Badge](https://img.shields.io/x)](https://site) Read on")).toBe("Read on");
  });

  it("keeps ordinary links", () => {
    expect(cleanReadme("See [the paper](https://arxiv.org/abs/1).")).toBe("See [the paper](https://arxiv.org/abs/1).");
  });

  it("leaves code fences untouched", () => {
    const md = "Intro <b>x</b>\n\n```html\n<div><img src=a></div>\n```\n\nAfter";
    expect(cleanReadme(md)).toBe("Intro x\n\n```html\n<div><img src=a></div>\n```\n\nAfter");
  });

  it("collapses the blank lines that removals leave", () => {
    expect(cleanReadme("a\n\n\n\n![x](y)\n\n\n\nb")).toBe("a\n\nb");
  });
});
