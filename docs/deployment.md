# Deployment

migadu-cli is a command-line tool, not a service. "Deploying" means publishing a release that people install, plus
publishing the project site.

## Environments and promotion path
| Environment | Purpose | Promotion from | Approval |
| --- | --- | --- | --- |
| Pull request | CI on the merge result: `verify` on 3 OSes × Node 22/24, CodeQL, Gitleaks, Semgrep, dependency review | feature branch | all 9 required checks green, conversations resolved |
| `main` | Always releasable | pull request | squash or rebase merge (linear history), no direct pushes, applies to admins |
| GitHub Pages | Project site at `https://ifthenelse.github.io/migadu-cli/` | `main`, when `site/**` or `.github/assets/**` change | automatic (`pages.yml`) |
| GitHub Release | Downloadable binaries | a `v*` tag on `main` | the maintainer pushes the tag |

There is no custom domain; the site lives at the GitHub Pages project URL.

## Release
The version has one source: `version` in `package.json`.

1. Merge the changes for the release into `main`. Bump `version` in `package.json` in the same or an earlier pull request.
2. Tag the merge commit with the same number and push the tag:
   ```sh
   git switch main && git pull
   git tag v0.2.0 && git push origin v0.2.0
   ```
3. `.github/workflows/release.yml` then runs three jobs:
   - **verify:** checks the tag equals `package.json`'s version and runs `scripts/verify`.
   - **build:** compiles one stand-alone binary per target with Bun (pinned to 1.4.5): `darwin-arm64`, `darwin-x64`,
     `linux-x64`, `linux-arm64`, `windows-x64`. It smoke-tests the Linux binary (prints help, never echoes a test secret) and writes `SHA256SUMS`.
   - **publish:** creates the GitHub release with the binaries, checksums and generated notes. This is the only job with write permission.

Binaries embed the Bun runtime, so they are 87–115 MB. The Node-based install (`node bin/migadu.js`) stays supported and is
the reference behaviour; see [ADR 0005](adr/0005-standalone-binaries-with-bun.md).

## Smoke tests
After a release, on each OS you can reach:
```sh
./migadu-<target> --help                                    # prints usage
MIGADU_EMAIL=a@b.c MIGADU_API_KEY=x ./migadu-<target> api GET /nope   # error: GET /nope -> HTTP 404 (or 401), no key in the output
sha256sum -c SHA256SUMS --ignore-missing                    # on Linux; shasum -a 256 -c on macOS
```
The macOS binaries are not notarised. If macOS quarantines a downloaded file, clear it with `xattr -d com.apple.quarantine <file>`.

## Rollback
Conditions: a release is broken or leaks something. Steps:
1. Mark the GitHub release as a pre-release or delete it (the tag can stay).
2. Fix forward on `main`, bump the patch version, tag again. Do not move or reuse a published tag.
3. If a secret may have been exposed, follow the [leaked credential runbook](runbooks/leaked-credential.md).
