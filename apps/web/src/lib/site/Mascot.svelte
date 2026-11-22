<script lang="ts">
  import { lookAt } from "./look.js";

  // Eight small characters with their own faces. Some have eyes that follow the pointer and blink,
  // some squint happily, one is sleepy. They are decoration, hidden from assistive tech, and with
  // reduced motion they hold still.
  export type Kind = "block" | "star" | "blob" | "round" | "cloud" | "drop" | "pill" | "triangle";
  export type Tone = "blue" | "orange" | "sand";
  let { kind, tone, class: className = "" }: { kind: Kind; tone?: Tone; class?: string } = $props();

  const defaults: Record<Kind, Tone> = { block: "blue", star: "orange", blob: "sand", round: "orange", cloud: "blue", drop: "orange", pill: "sand", triangle: "blue" };
  const colours = $derived(
    {
      blue: ["var(--mascot-blue)", "var(--mascot-blue-deep)"],
      orange: ["var(--mascot-orange)", "var(--mascot-orange-deep)"],
      sand: ["var(--mascot-sand)", "var(--mascot-sand-deep)"],
    }[tone ?? defaults[kind]],
  );
  const fill = $derived(colours[0]);
  const deep = $derived(colours[1]);

  const star = (() => {
    const pts: string[] = [];
    for (let i = 0; i < 10; i++) {
      const r = i % 2 === 0 ? 42 : 21;
      const a = (Math.PI / 5) * i - Math.PI / 2;
      pts.push(`${(60 + r * Math.cos(a)).toFixed(1)},${(54 + r * Math.sin(a)).toFixed(1)}`);
    }
    return pts.join(" ");
  })();
  const blob = "M18 62 C12 38 32 16 62 18 C94 20 110 44 102 68 C96 88 68 96 44 92 C28 90 20 78 18 62 Z";
  const cloud = "M32 86 C12 86 8 60 28 56 C26 34 50 24 60 40 C68 24 96 30 96 52 C114 54 114 86 92 86 Z";
  const drop = "M60 8 C80 38 102 54 102 76 C102 96 84 102 60 102 C36 102 18 96 18 76 C18 54 40 38 60 8 Z";
  const triangle = "60,20 100,88 20,88";
  const eyes = "transform: translate(calc(var(--ex, 0) * 1px), calc(var(--ey, 0) * 1px))";
  const ink = "var(--foreground)";
</script>

<svg viewBox="0 0 120 108" class="mascot {kind === 'star' ? 'mascot-bob' : ''} {className}" aria-hidden="true" use:lookAt={4}>
  {#if kind === "block"}
    <g transform="translate(5 5)"><rect x="14" y="16" width="92" height="76" rx="16" fill={deep} /></g>
    <rect x="14" y="16" width="92" height="76" rx="16" fill={fill} />
    <g class="mascot-eyes" style={eyes}>
      <ellipse class="mascot-blink" cx="44" cy="54" rx="5" ry="9" fill={ink} />
      <ellipse class="mascot-blink" cx="72" cy="54" rx="5" ry="9" fill={ink} />
    </g>
  {:else if kind === "star"}
    <g transform="translate(5 5)"><polygon points={star} fill={deep} stroke={deep} stroke-width="12" stroke-linejoin="round" /></g>
    <polygon points={star} fill={fill} stroke={fill} stroke-width="12" stroke-linejoin="round" />
    <path d="M43 56 q6 -9 12 0 M65 56 q6 -9 12 0" fill="none" stroke={ink} stroke-width="4" stroke-linecap="round" />
  {:else if kind === "blob"}
    <g transform="translate(5 5)"><path d={blob} fill={deep} /></g>
    <path d={blob} fill={fill} />
    <path d="M38 58 h14 M68 58 h14" fill="none" stroke={ink} stroke-width="4" stroke-linecap="round" />
    <path d="M54 72 q6 5 12 0" fill="none" stroke={ink} stroke-width="3.5" stroke-linecap="round" />
  {:else if kind === "round"}
    <g transform="translate(5 5)"><circle cx="60" cy="54" r="40" fill={deep} /></g>
    <circle cx="60" cy="54" r="40" fill={fill} />
    <g class="mascot-eyes" style={eyes}>
      <ellipse class="mascot-blink" cx="46" cy="48" rx="4.5" ry="8" fill={ink} />
      <ellipse class="mascot-blink" cx="74" cy="48" rx="4.5" ry="8" fill={ink} />
    </g>
    <path d="M52 72 q8 7 16 0" fill="none" stroke={ink} stroke-width="3.5" stroke-linecap="round" />
  {:else if kind === "cloud"}
    <g transform="translate(5 5)"><path d={cloud} fill={deep} /></g>
    <path d={cloud} fill={fill} />
    <g class="mascot-eyes" style={eyes}>
      <circle class="mascot-blink" cx="46" cy="62" r="4.5" fill={ink} />
      <circle class="mascot-blink" cx="72" cy="62" r="4.5" fill={ink} />
    </g>
    <path d="M52 74 q7 6 14 0" fill="none" stroke={ink} stroke-width="3.5" stroke-linecap="round" />
  {:else if kind === "drop"}
    <g transform="translate(5 5)"><path d={drop} fill={deep} /></g>
    <path d={drop} fill={fill} />
    <g class="mascot-eyes" style={eyes}>
      <ellipse class="mascot-blink" cx="46" cy="68" rx="4.5" ry="8" fill={ink} />
      <ellipse class="mascot-blink" cx="74" cy="68" rx="4.5" ry="8" fill={ink} />
    </g>
    <ellipse cx="60" cy="86" rx="4" ry="3.2" fill="none" stroke={ink} stroke-width="3.2" />
  {:else if kind === "pill"}
    <g transform="translate(5 5)"><rect x="34" y="8" width="52" height="94" rx="26" fill={deep} /></g>
    <rect x="34" y="8" width="52" height="94" rx="26" fill={fill} />
    <path d="M46 46 q5 -8 10 0 M64 46 q5 -8 10 0" fill="none" stroke={ink} stroke-width="3.8" stroke-linecap="round" />
    <path d="M54 62 q6 5 12 0" fill="none" stroke={ink} stroke-width="3.5" stroke-linecap="round" />
  {:else}
    <g transform="translate(5 5)"><polygon points={triangle} fill={deep} stroke={deep} stroke-width="14" stroke-linejoin="round" /></g>
    <polygon points={triangle} fill={fill} stroke={fill} stroke-width="14" stroke-linejoin="round" />
    <g class="mascot-eyes" style={eyes}>
      <ellipse class="mascot-blink" cx="50" cy="66" rx="4" ry="7.5" fill={ink} />
      <ellipse class="mascot-blink" cx="70" cy="66" rx="4" ry="7.5" fill={ink} />
    </g>
    <path d="M54 78 q6 5 12 0" fill="none" stroke={ink} stroke-width="3.4" stroke-linecap="round" />
  {/if}
</svg>
