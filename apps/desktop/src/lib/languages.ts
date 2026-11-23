const names = new Intl.DisplayNames(["en"], { type: "language" });

/** "de" as "German"; unknown codes come back unchanged. */
export function languageName(code: string): string {
  try {
    return names.of(code) ?? code;
  } catch {
    return code;
  }
}

/** A short line for a card: the names when there are few, otherwise a count. */
export function languageSummary(codes: readonly string[]): string {
  if (codes.length === 0) return "";
  if (codes.length <= 3) return codes.map(languageName).join(", ");
  return `${codes.length} languages`;
}
