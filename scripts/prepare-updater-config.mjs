import { readFile, writeFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
const templateUrl = new URL("src-tauri/tauri.release.conf.json", root);
const outputUrl = new URL("src-tauri/tauri.release.generated.conf.json", root);
const publicKey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();

if (!publicKey) {
  throw new Error(
    "TAURI_UPDATER_PUBLIC_KEY is missing. Configure the repository variable described in docs/RELEASING.md.",
  );
}

if (publicKey.length < 40) {
  throw new Error("TAURI_UPDATER_PUBLIC_KEY is too short to be a valid Tauri updater public key.");
}

const config = JSON.parse(await readFile(templateUrl, "utf8"));
config.plugins.updater.pubkey = publicKey;
await writeFile(outputUrl, `${JSON.stringify(config, null, 2)}\n`, "utf8");
console.log("Prepared release-only updater configuration.");
