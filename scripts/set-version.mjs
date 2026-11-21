// Sets the app version everywhere it is written, from a release tag such as v0.1.0-pre.1.
// Installers and the macOS bundle need a plain x.y.z, so a pre-release suffix is dropped from
// the app version and kept in the tag and the release name.
import { readFileSync, writeFileSync } from "node:fs";

const tag = process.argv[2] ?? "";
const match = /^v?(\d+\.\d+\.\d+)(?:-[0-9A-Za-z.-]+)?$/.exec(tag);
if (!match) {
  console.error(`"${tag}" is not a release tag. Use something like v0.1.0 or v0.1.0-pre.1.`);
  process.exit(1);
}
const version = match[1];

const edit = (path, change) => writeFileSync(path, change(readFileSync(path, "utf8")));

edit("package.json", (s) => s.replace(/("version":\s*")[^"]+(")/, `$1${version}$2`));
edit("src-tauri/tauri.conf.json", (s) => s.replace(/("version":\s*")[^"]+(")/, `$1${version}$2`));
// Only the package's own version, the first `version = ` line under [package].
edit("src-tauri/Cargo.toml", (s) => s.replace(/^version = ".*"$/m, `version = "${version}"`));

console.log(version);
