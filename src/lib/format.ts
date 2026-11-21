export function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
  if (bytes >= 1_048_576) return `${Math.round(bytes / 1_048_576)} MB`;
  return `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** A rough time left, or null while the speed is still unknown. */
export function formatEta(remainingBytes: number, bytesPerSecond: number): string | null {
  if (bytesPerSecond < 1024 || remainingBytes <= 0) return null;
  const seconds = remainingBytes / bytesPerSecond;
  if (seconds < 60) return "under a minute left";
  if (seconds < 3600) return `about ${Math.round(seconds / 60)} min left`;
  return `about ${(seconds / 3600).toFixed(1)} h left`;
}

export function formatDate(ms: number): string {
  return new Date(ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
