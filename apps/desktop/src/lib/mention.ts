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

/** The text with the open `@word` reduced to a bare `@`, so the list stays open for the next step. */
export function withBareMention(text: string): string {
  return text.replace(OPEN_WITH_LEAD, "$1@");
}

export type CategoryId = "recording" | "transcript" | "note" | "summary" | "image" | "file";

export type Category = {
  id: CategoryId;
  label: string;
  /** Words that jump straight into this category: `@recordings`, `@notes`, `@files`. */
  aliases: string[];
};

// Decision (product): mentions are grouped the way people think of their things. `@` shows the
// groups, `@rec` or `@notes` jumps into one, and any other word searches everything.
export const CATEGORIES: Category[] = [
  { id: "recording", label: "Recordings", aliases: ["recordings", "recording", "dictations", "dictation", "voice"] },
  { id: "transcript", label: "Transcripts", aliases: ["transcripts", "transcript", "audio"] },
  { id: "note", label: "Notes", aliases: ["notes", "note"] },
  { id: "summary", label: "Summaries", aliases: ["summaries", "summary"] },
  { id: "image", label: "Images", aliases: ["images", "image", "pictures", "photos"] },
  { id: "file", label: "Files", aliases: ["files", "file", "folders", "folder", "documents"] },
];

/** The group a typed word names exactly, such as `notes`. */
export function categoryFor(query: string): Category | null {
  const q = query.trim().toLowerCase();
  if (!q) return null;
  return CATEGORIES.find((c) => c.aliases.includes(q)) ?? null;
}

/** The groups a partial word could still become: `re` offers Recordings, `s` offers Summaries. */
export function categoriesMatching(query: string): Category[] {
  const q = query.trim().toLowerCase();
  if (!q) return CATEGORIES;
  return CATEGORIES.filter((c) => c.aliases.some((a) => a.startsWith(q)));
}
