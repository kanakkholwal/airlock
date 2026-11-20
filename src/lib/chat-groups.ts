export type Group<T> = { label: string; items: T[] };

const DAY = 24 * 60 * 60 * 1000;

/**
 * Decision (product): chats are grouped by how recently they were used, so the list stays
 * scannable without the user filing anything. Input order is kept inside each group.
 */
export function groupByRecency<T extends { updated_at: number }>(
  items: readonly T[],
  now: number = Date.now(),
): Group<T>[] {
  const startOfToday = new Date(now).setHours(0, 0, 0, 0);
  const buckets: { label: string; since: number; items: T[] }[] = [
    { label: "Today", since: startOfToday, items: [] },
    { label: "Yesterday", since: startOfToday - DAY, items: [] },
    { label: "Previous 7 days", since: startOfToday - 7 * DAY, items: [] },
    { label: "Previous 30 days", since: startOfToday - 30 * DAY, items: [] },
    { label: "Older", since: Number.NEGATIVE_INFINITY, items: [] },
  ];
  for (const item of items) {
    buckets.find((b) => item.updated_at >= b.since)?.items.push(item);
  }
  return buckets.filter((b) => b.items.length > 0).map(({ label, items }) => ({ label, items }));
}
