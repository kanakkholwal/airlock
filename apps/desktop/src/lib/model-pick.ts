type Candidate = { id: string; recommended: boolean };

/**
 * Decision (product): which downloaded model loads without asking. The last one you used wins,
 * then the recommended one, then whatever is on disk, so a launch never waits for a choice.
 */
export function pickAutoModel<T extends Candidate>(downloaded: readonly T[], lastId: string | null): T | undefined {
  return downloaded.find((m) => m.id === lastId) ?? downloaded.find((m) => m.recommended) ?? downloaded[0];
}
