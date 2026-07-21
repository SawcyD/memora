import { readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
const read = (path) => readFile(new URL(path, root), "utf8");
const fail = (message) => {
  console.error(`Version check failed: ${message}`);
  process.exitCode = 1;
};

const [packageText, lockText, cargoText, cargoLockText, tauriText, aboutText] = await Promise.all([
  read("package.json"),
  read("package-lock.json"),
  read("src-tauri/Cargo.toml"),
  read("src-tauri/Cargo.lock"),
  read("src-tauri/tauri.conf.json"),
  read("src/pages/About.tsx"),
]);

const packageJson = JSON.parse(packageText);
const packageLock = JSON.parse(lockText);
const tauriConfig = JSON.parse(tauriText);
const expected = packageJson.version;
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

if (!semver.test(expected)) fail(`package.json contains invalid SemVer ${JSON.stringify(expected)}`);

const cargoVersion = cargoText.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const cargoLockVersion = cargoLockText.match(/\[\[package\]\]\s+name = "memora"\s+version = "([^"]+)"/)?.[1];
const aboutVersion = aboutText.match(/<InfoRow label="Version" value="([^"]+)"/m)?.[1];

const locations = [
  ["package-lock.json root", packageLock.version],
  ["package-lock.json workspace root", packageLock.packages?.[""]?.version],
  ["src-tauri/Cargo.toml", cargoVersion],
  ["src-tauri/Cargo.lock (memora package)", cargoLockVersion],
  ["src-tauri/tauri.conf.json", tauriConfig.version],
  ["src/pages/About.tsx", aboutVersion],
];

for (const [location, version] of locations) {
  if (!version) fail(`${location} does not expose a recognizable version`);
  else if (version !== expected) fail(`${location} is ${version}; expected ${expected}`);
}

if (!process.exitCode) console.log(`All Memora version locations agree on ${expected}.`);
