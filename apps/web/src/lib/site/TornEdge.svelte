<script lang="ts">
  // A cut-paper edge. The jagged line is the same every time (seeded), so a page never
  // reshuffles between renders. `fill` takes any CSS colour; `flip` turns it to the bottom edge.
  let { fill = "var(--card)", flip = false, height = 22 }: { fill?: string; flip?: boolean; height?: number } = $props();

  const W = 1200;
  const path = (() => {
    let seed = 7;
    const rnd = () => {
      seed = (seed * 16807) % 2147483647;
      return seed / 2147483647;
    };
    let d = `M0 ${height}`;
    let x = 0;
    while (x < W) {
      x += 14 + rnd() * 26;
      d += ` L${Math.min(x, W).toFixed(1)} ${(rnd() * height * 0.75).toFixed(1)}`;
    }
    return `${d} L${W} ${height} Z`;
  })();
</script>

<svg
  viewBox={`0 0 ${W} ${height}`}
  preserveAspectRatio="none"
  aria-hidden="true"
  class="block w-full"
  style:height="{height}px"
  style:transform={flip ? "scaleY(-1)" : undefined}
>
  <path d={path} {fill} />
</svg>
