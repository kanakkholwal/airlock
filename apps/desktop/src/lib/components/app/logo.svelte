<script lang="ts">
  let { size = 20 }: { size?: number } = $props();
</script>

<!-- The Airlock mark: the same drawing as branding/icon.svg. -->
<svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true">
  <defs>
    <linearGradient id="logo-bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#2a2a2e" />
      <stop offset="1" stop-color="#0f0f11" />
    </linearGradient>
  </defs>
  <rect x="64" y="64" width="896" height="896" rx="208" fill="url(#logo-bg)" />
  <path d="M300 790 L512 290 L724 790" fill="none" stroke="#f4f4f5" stroke-width="88" stroke-linecap="round" stroke-linejoin="round" />
  <path d="M410 640 H614" fill="none" stroke="#f4f4f5" stroke-width="72" stroke-linecap="round" />
  <circle cx="512" cy="196" r="46" fill="#ef4444" />
</svg>
