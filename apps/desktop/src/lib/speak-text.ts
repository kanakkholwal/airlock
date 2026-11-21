/**
 * Turns a markdown reply into text that sounds right when spoken: code blocks are skipped,
 * links keep only their words, and formatting marks are dropped, so nothing is read out as
 * "asterisk asterisk" or a URL.
 */
export function plainForSpeech(markdown: string): string {
  return markdown
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`([^`]*)`/g, "$1")
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .replace(/^\s{0,3}#{1,6}\s+/gm, "")
    .replace(/^\s*>\s?/gm, "")
    .replace(/^\s*(?:[-*+]|\d+[.)])\s+/gm, "")
    .replace(/(\*\*|__|\*|_|~~)/g, "")
    .replace(/https?:\/\/\S+/g, "")
    .replace(/[|]/g, " ")
    .replace(/\n{2,}/g, ". ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Splits long text at sentence ends, since some engines cut off very long utterances. */
export function chunkForSpeech(text: string, max = 220): string[] {
  const sentences = text.match(/[^.!?]+[.!?]*\s*/g) ?? [text];
  const chunks: string[] = [];
  let current = "";
  for (const s of sentences) {
    if (current && current.length + s.length > max) {
      chunks.push(current.trim());
      current = "";
    }
    current += s;
  }
  if (current.trim()) chunks.push(current.trim());
  return chunks;
}
