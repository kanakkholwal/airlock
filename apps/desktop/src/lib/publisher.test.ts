import { describe, expect, it } from "vitest";
import { hfUrl, hueOf, modelNameOf, publisherOf } from "./publisher";

describe("publisher helpers", () => {
  it("splits a repo id", () => {
    expect(publisherOf("Qwen/Qwen3-4B-GGUF")).toBe("Qwen");
    expect(modelNameOf("Qwen/Qwen3-4B-GGUF")).toBe("Qwen3-4B-GGUF");
    expect(publisherOf("local-model")).toBe("");
    expect(hfUrl("a/b")).toBe("https://huggingface.co/a/b");
  });
  it("gives one hue per name, ignoring case", () => {
    expect(hueOf("Qwen")).toBe(hueOf("qwen"));
    expect(hueOf("Qwen")).not.toBe(hueOf("unsloth"));
    expect(hueOf("x")).toBeLessThan(360);
  });
});
