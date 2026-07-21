# Releasing Memora

Memora uses Semantic Versioning and GitHub Releases as the distribution endpoint for its Windows installer and Tauri updater metadata.

The updater reads:

```text
https://github.com/SawcyD/memora/releases/latest/download/latest.json
```

`latest.json` is not committed to the repository. The trusted release workflow generates it from the signed NSIS installer and its `.sig` file, then uploads all three to the matching GitHub Release.

## One-time repository setup

Generate one updater signing key pair on a trusted machine:

```powershell
npm run tauri signer generate -- -w "$HOME\.tauri\memora.key"
```

Keep this original key pair for the lifetime of the application. Replacing it after users install Memora prevents those installations from trusting future updates.

Configure the GitHub `release` environment and add:

| Type | Name | Value |
| --- | --- | --- |
| Actions secret | `TAURI_SIGNING_PRIVATE_KEY` | Complete contents of `memora.key` |
| Actions secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Password chosen during key generation; omit only for an unencrypted key |
| Actions variable | `TAURI_UPDATER_PUBLIC_KEY` | Complete public key printed by the signer command |

The private key must never be committed, attached to an issue, added to a pull request, or shared with contributors. The public key is safe to expose; the workflow injects it into a release-only Tauri configuration so ordinary fork builds do not require repository variables.

Recommended repository settings:

1. Allow GitHub Actions to create and approve releases with `Read and write permissions` for `GITHUB_TOKEN`.
2. Create an environment named `release` and limit it to the default branch.
3. Protect `main` and require the `Validate Windows application` check before merging.
4. Require pull requests for external contributions.

## Publishing a normal release

Update the version once in every location required by `AGENTS.md`:

```text
package.json
package-lock.json
src-tauri/Cargo.toml
src-tauri/Cargo.lock
src-tauri/tauri.conf.json
src/pages/About.tsx
```

Run the local checks:

```powershell
npm run version:check
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
```

Open a pull request with the version change. Fork pull requests run `.github/workflows/ci.yml` without secrets or write permissions. After the version change merges to `main`, `.github/workflows/release.yml`:

1. Verifies all version locations contain the same valid SemVer.
2. Refuses to overwrite an existing tag on an automatic push release.
3. Builds the signed Windows NSIS installer and updater signature.
4. Creates tag `v<version>` and a published GitHub Release.
5. Uploads the installer, `.sig`, and generated `latest.json`.

Only commits touching a synchronized version location trigger an automatic release. Ordinary merges still run CI but do not publish.

## Manual and pre-release paths

Use **Actions → Release → Run workflow** to retry or manually create the release for the current version. Manual runs may update an existing matching release, which is useful after correcting repository configuration.

Alternatively, create and publish a GitHub Release whose tag exactly matches `v<package.json version>`. The published-release event builds and attaches the signed assets. A prerelease receives its own `latest.json`, but the stable application endpoint continues to follow GitHub's latest non-prerelease release.

## How `latest.json` stays correct

Tauri Action reads the application version and the updater artifacts produced by `createUpdaterArtifacts`. It embeds the actual signature contents and exact GitHub download URL in the static JSON. Generating this file before the signed artifact exists would produce unusable updater metadata, so it is intentionally created during the release job rather than edited by hand.
