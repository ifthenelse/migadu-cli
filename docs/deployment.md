# Deployment

migadu-cli is a command-line tool, not a service. "Deploying" means publishing a release that people install, plus
publishing the project site.

## Environments and promotion path
| Environment | Purpose | Promotion from | Approval |
| --- | --- | --- | --- |
| Pull request | CI on the merge result: `verify` on 3 OSes (stable Rust) plus an MSRV build, CodeQL, Gitleaks, Semgrep, dependency review | feature branch | all 9 required checks green, conversations resolved |
| `main` | Always releasable | pull request | squash or rebase merge (linear history), no direct pushes, applies to admins |
| GitHub Pages | Project site at `https://ifthenelse.github.io/migadu-cli/` | `main`, when `site/**` or `.github/assets/**` change | automatic (`pages.yml`) |
| GitHub Release | Downloadable archives, source package, checksums | a change to the version in `Cargo.toml` on `main` | merging the pull request that bumps the version |

There is no custom domain; the site lives at the GitHub Pages project URL.

## Release
The version has one source: `version` in `Cargo.toml` (`Cargo.lock` follows it), and it follows [Semantic Versioning 2.0.0](https://semver.org/). Every pull request bumps it:
MAJOR for a backwards-incompatible change to commands, options, output or configuration, MINOR for backwards-compatible
functionality, PATCH for fixes, documentation and CI. While the major version is 0, bump MINOR for breaking changes.

1. Bump `version` in `Cargo.toml` in the pull request that makes the change (run `cargo build` so `Cargo.lock` follows), and merge it into `main`.
2. When a push to `main` changes `Cargo.toml`, `.github/workflows/release.yml` runs by itself (it can also be started
   by hand from the Actions tab, or by pushing a `v<version>` tag). Its jobs:
   - **plan:** reads the version, derives the tag `v<version>`, and stops quietly if a release with that tag already exists.
     A pushed tag that differs from `Cargo.toml` fails.
   - **verify:** runs `scripts/verify` (format, clippy, tests).
   - **build:** compiles natively on each operating system, in release mode with `--locked`: `darwin-arm64`, `darwin-x64`
     (cross-built on the Arm runner), `linux-x64`, `linux-arm64` and `windows-x64`. It smoke-tests every binary the runner can
     execute (prints help, never echoes a test secret) and archives each with `LICENSE` and `README.md` (`.tar.gz`, or `.zip`
     for Windows).
   - **publish:** adds the source package from `cargo package` and `SHA256SUMS`, then creates the tag on the merged commit
     and the GitHub release with generated notes. This is the only job with write permission.

Because the trigger is a changed `Cargo.toml`, a merged pull request that bumps the version is published automatically.
Pull requests that do not change the version publish nothing. Tags created by the workflow itself do not start a second run.

Binaries are about 1.4 MB, statically linked against rustls, and need no runtime. Linux binaries link the system C library
and are built on Ubuntu 22.04 for wide compatibility. See [ADR 0006](adr/0006-rewrite-in-rust.md).

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
2. Fix forward on `main` with a pull request that bumps the patch version; the workflow releases it. Do not move or reuse a published tag.
3. If a secret may have been exposed, follow the [leaked credential runbook](runbooks/leaked-credential.md).
