// Frees disk: removes caches, incremental dirs and optionally the whole cargo target.
import { rmSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

for (const p of [".svelte-kit", "node_modules/.cache", "node_modules/.vite", "build"]) {
  rmSync(p, { recursive: true, force: true });
}

const walk = (dir) => {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (!statSync(full).isDirectory()) continue;
    if (name === "incremental") rmSync(full, { recursive: true, force: true });
    else walk(full);
  }
};
try {
  walk("src-tauri/target");
} catch {}
if (process.argv.includes("--all")) rmSync("src-tauri/target", { recursive: true, force: true });
