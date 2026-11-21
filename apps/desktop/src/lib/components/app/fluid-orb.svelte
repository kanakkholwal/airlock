<script lang="ts">
  import { cn } from "#lib/cn.js";

  // Decision (product): a soft fluid orb. `activity` (0 to 1) speeds the flow and the colour
  // drift while someone speaks or is spoken to, and the glow under it takes the colour on show.
  // Port of a React FluidOrb with those additions.
  let {
    size = 160,
    color = "#1A73F2",
    colors,
    activity = 0,
    glow = true,
    label,
    class: className,
  }: {
    size?: number;
    color?: string;
    /** Two or more colours to drift between; overrides `color`. */
    colors?: string[];
    activity?: number;
    glow?: boolean;
    label?: string;
    class?: string;
  } = $props();

  let wrap = $state<HTMLDivElement>();

  const VERT = `attribute vec2 a_pos; void main(){ gl_Position = vec4(a_pos, 0.0, 1.0); }`;
  const FRAG = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_color;
float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123); }
float noise(vec2 p){
  vec2 i = floor(p); vec2 f = fract(p); vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}
float fbm(vec2 p){
  float v = 0.0; float a = 0.6;
  for (int i = 0; i < 3; i++) { v += a * noise(p); p *= 2.0; a *= 0.5; }
  return v;
}
void main(){
  vec2 uv = gl_FragCoord.xy / u_resolution.xy;
  float t = u_time * 0.22;
  vec2 drift = vec2(sin(t) + 0.6 * sin(t * 1.7 + 1.3), cos(t * 0.8) + 0.6 * cos(t * 1.3 + 2.1));
  vec2 p = vec2(uv.x * 1.8, uv.y) + drift * 0.7;
  vec2 q = vec2(fbm(p + drift), fbm(p + vec2(3.2, 1.5) - drift));
  float f = fbm(p + 1.2 * q);
  float g = clamp(1.0 - uv.y, 0.0, 1.0);
  float anchor = smoothstep(0.0, 0.3, uv.y);
  float shade = clamp(g + (f - 0.5) * 0.8 * anchor, 0.0, 1.0);
  vec3 white = vec3(0.99, 1.0, 1.0);
  vec3 light = mix(white, u_color, 0.5);
  vec3 col = mix(white, light, smoothstep(0.28, 0.52, shade));
  col = mix(col, u_color, smoothstep(0.58, 0.88, shade));
  float edge = smoothstep(0.5, 0.49, distance(uv, vec2(0.5)));
  gl_FragColor = vec4(col * edge, edge);
}`;

  function hexToRgb(hex: string): [number, number, number] {
    let h = hex.replace("#", "").trim();
    if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2];
    const n = parseInt(h, 16);
    if (h.length !== 6 || Number.isNaN(n)) return [0.1, 0.45, 0.95];
    return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
  }

  function compile(gl: WebGLRenderingContext, type: number, src: string) {
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, src);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  let canvas = $state<HTMLCanvasElement>();
  // Read each frame so activity changes never restart the GL setup.
  let live = 0;
  $effect(() => {
    live = activity;
  });

  $effect(() => {
    const el = canvas;
    if (!el) return;
    const gl = el.getContext("webgl", { antialias: true, alpha: true });
    if (!gl) return;
    const program = gl.createProgram();
    const vert = compile(gl, gl.VERTEX_SHADER, VERT);
    const frag = compile(gl, gl.FRAGMENT_SHADER, FRAG);
    if (!program || !vert || !frag) return;
    gl.attachShader(program, vert);
    gl.attachShader(program, frag);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) return;
    gl.useProgram(program);

    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl.STATIC_DRAW);
    const aPos = gl.getAttribLocation(program, "a_pos");
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

    const uTime = gl.getUniformLocation(program, "u_time");
    const palette = (colors && colors.length > 0 ? colors : [color]).map(hexToRgb);
    const uColor = gl.getUniformLocation(program, "u_color");
    // Moves along the palette and wraps, blending the two colours either side of the position.
    const at = (phase: number): [number, number, number] => {
      if (palette.length === 1) return palette[0];
      const p = ((phase % palette.length) + palette.length) % palette.length;
      const i = Math.floor(p);
      const t = p - i;
      const a = palette[i];
      const c = palette[(i + 1) % palette.length];
      return [a[0] + (c[0] - a[0]) * t, a[1] + (c[1] - a[1]) * t, a[2] + (c[2] - a[2]) * t];
    };
    let phase = 0;

    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const px = Math.round(size * dpr);
    el.width = px;
    el.height = px;
    gl.viewport(0, 0, px, px);
    gl.uniform2f(gl.getUniformLocation(program, "u_resolution"), px, px);

    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let clock = 0;
    let last = performance.now();
    let raf = 0;
    const render = (now: number) => {
      const dt = Math.min((now - last) / 1000, 0.05);
      last = now;
      clock += reduce ? 0 : dt * (1 + live * 4);
      phase += reduce ? 0 : dt * (0.03 + live * 0.22);
      const [r, g, b] = at(phase);
      gl.uniform3f(uColor, r, g, b);
      wrap?.style.setProperty("--orb", `${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)}`);
      gl.uniform1f(uTime, clock);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      if (!reduce) raf = requestAnimationFrame(render);
    };
    render(last);

    return () => {
      cancelAnimationFrame(raf);
      gl.deleteProgram(program);
      gl.deleteShader(vert);
      gl.deleteShader(frag);
      gl.deleteBuffer(buffer);
    };
  });
</script>

<div
  bind:this={wrap}
  data-slot="fluid-orb"
  class={cn("relative", className)}
  style:width="{size}px"
  style:height="{size}px"
  style:--orb="26 115 242"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : true}
>
  {#if glow}
    <span
      aria-hidden="true"
      class="pointer-events-none absolute inset-3 translate-y-[14%] rounded-full blur-2xl transition-opacity duration-700"
      style:background="rgb(var(--orb))"
      style:opacity={0.28 + activity * 0.32}
    ></span>
  {/if}
  <div class="relative size-full overflow-hidden rounded-full">
    <canvas bind:this={canvas} class="h-full w-full"></canvas>
  </div>
</div>
