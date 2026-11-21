// Rewrites `#lib/...` imports inside the Baby UI component folders into relative paths, so the
// folders can move into a shared package (and be reused by the website) without edits.
// The registry installs new components with `#lib` aliases, so run this after each install:
//   pnpm ui:relativize
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, posix, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const lib = join(root, "src", "lib");
const folders = ["components/ui", "components/agents", "components/text"].map((f) => join(lib, f));

// from "#lib/x", import("#lib/x") and bare import "#lib/x"
const pattern = /((?:from|import)\s*\(?\s*)(["'])#lib\/([^"']+)\2/g;

function* files(dir) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) yield* files(full);
    else if (/\.(svelte|ts)$/.test(name)) yield full;
  }
}

let changed = 0;
for (const folder of folders) {
  for (const file of files(folder)) {
    const source = readFileSync(file, "utf8");
    const next = source.replace(pattern, (_, lead, quote, target) => {
      let rel = relative(dirname(file), join(lib, target)).split(sep).join(posix.sep);
      if (!rel.startsWith(".")) rel = `./${rel}`;
      return `${lead}${quote}${rel}${quote}`;
    });
    if (next !== source) {
      writeFileSync(file, next);
      changed++;
    }
  }
}
console.log(`Rewrote ${changed} file${changed === 1 ? "" : "s"}.`);
