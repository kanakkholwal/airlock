<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<!-- The Airlock mark, redrawn in the site's ink and accent. -->
<svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true">
  <rect x="64" y="64" width="896" height="896" rx="208" fill="var(--foreground)" />
  <path d="M300 790 L512 290 L724 790" fill="none" stroke="var(--background)" stroke-width="88" stroke-linecap="round" stroke-linejoin="round" />
  <path d="M410 640 H614" fill="none" stroke="var(--background)" stroke-width="72" stroke-linecap="round" />
  <circle cx="512" cy="196" r="46" fill="var(--primary)" />
</svg>
