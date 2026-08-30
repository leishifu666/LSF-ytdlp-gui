// Generates the Tauri updater metadata file (latest.json) for a release.
//
// Usage:
//   node scripts/gen-latest-json.mjs <version> <path/to/setup.exe.sig> <asset-filename> <out/latest.json> [notes]
//
// The `signature` value is the full contents of the .sig file produced by
// `tauri build` (TAURI_SIGNING_PRIVATE_KEY must have been set).
import { readFileSync, writeFileSync } from "node:fs";

const [version, sigPath, assetName, outPath, notes = ""] =
  process.argv.slice(2);
if (!version || !sigPath || !assetName || !outPath) {
  console.error(
    "usage: node scripts/gen-latest-json.mjs <version> <sig> <assetName> <out> [notes]",
  );
  process.exit(1);
}

const repo = "leishifu666/LSF-ytdlp-gui";
const url = `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(assetName)}`;

const json = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: readFileSync(sigPath, "utf8").trim(),
      url,
    },
  },
};

writeFileSync(outPath, JSON.stringify(json, null, 2) + "\n");
console.log(`wrote ${outPath}`);
console.log(`  version: ${version}`);
console.log(`  url:     ${url}`);
