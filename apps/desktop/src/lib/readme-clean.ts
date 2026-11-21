// Decision (product): a repository README is untrusted and noisy. Only plain markdown text is
// shown: HTML, comments, images and badges are removed (an image would be a request to a third
// party), and what is inside code fences is left exactly as written.
const FENCE = /(^|\n)(```|~~~)[^\n]*\n[\s\S]*?(\n\2[^\n]*(?=\n|$)|$)/g;

function cleanProse(text: string): string {
  return (
    text
      // HTML comments, then any tag (so <div>, <img>, <a>, <br> and the like vanish, text stays).
      .replace(/<!--[\s\S]*?-->/g, "")
      .replace(/<\/?[a-zA-Z][^>]*>/g, "")
      // Markdown images, including ones used as badges inside links, and reference-style ones.
      .replace(/!\[[^\]]*\]\([^)]*\)/g, "")
      .replace(/!\[[^\]]*\]\[[^\]]*\]/g, "")
      // A link left with no text after its image went away.
      .replace(/\[\s*\]\([^)]*\)/g, "")
      // Definitions that only existed for removed images or links are noise.
      .replace(/^\[[^\]]+\]:\s*\S+.*$/gm, "")
      .replace(/[ \t]+$/gm, "")
      .replace(/\n{3,}/g, "\n\n")
  );
}

export function cleanReadme(source: string): string {
  let out = "";
  let last = 0;
  for (const m of source.matchAll(FENCE)) {
    const start = m.index ?? 0;
    out += cleanProse(source.slice(last, start)) + m[0];
    last = start + m[0].length;
  }
  out += cleanProse(source.slice(last));
  return out.trim();
}
