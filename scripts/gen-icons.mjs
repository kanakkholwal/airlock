// Builds app icons and tray glyphs from branding/*.svg via `tauri icon` (resvg).
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const tauri = (args) =>
  execFileSync("pnpm", ["tauri", "icon", ...args], { stdio: "inherit", shell: true });

tauri(["branding/icon.svg", "-o", "src-tauri/icons"]);

const glyph = (ink, beacon) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
<path d="M250 900 L512 300 L774 900" fill="none" stroke="${ink}" stroke-width="110" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M390 700 H634" fill="none" stroke="${ink}" stroke-width="90" stroke-linecap="round"/>
${beacon}</svg>`;
const dot = (c) => `<circle cx="512" cy="150" r="80" fill="${c}"/>`;
const ring = (c) => `<circle cx="512" cy="150" r="64" fill="none" stroke="${c}" stroke-width="40"/>`;

const state = {
  idle: (ink) => dot(ink),
  recording: () => dot("#ef4444"),
  transcribing: (ink) => ring(ink),
  idle_warning: () => dot("#f59e0b"),
};
const themes = { "": "#f4f4f5", _dark: "#18181b" };
const tmp = mkdtempSync(join(tmpdir(), "airlock-tray-"));
for (const [name, beacon] of Object.entries(state)) {
  for (const [suffix, ink] of Object.entries(themes)) {
    const file = `tray_${name}${suffix}`;
    const svg = join(tmp, `${file}.svg`);
    const out = join(tmp, file);
    mkdirSync(out);
    writeFileSync(svg, glyph(ink, beacon(ink)));
    tauri([svg, "-o", out]);
    copyFileSync(join(out, "64x64.png"), `src-tauri/resources/${file}.png`);
  }
}
