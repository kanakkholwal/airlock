/** The `@word` at the very end of the text that opens the mention list, or null when there is none. */
const OPEN = /(?:^|\s)@([^\s@]{0,30})$/;
const OPEN_WITH_LEAD = /(^|\s)@[^\s@]{0,30}$/;

export function mentionQuery(text: string): string | null {
  return OPEN.exec(text)?.[1] ?? null;
}

/** The text with that open `@word` taken out, keeping what came before it. */
export function withoutMention(text: string): string {
  return text.replace(OPEN_WITH_LEAD, "$1").trimEnd();
}
