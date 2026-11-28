# Deployment

migadu-cli is a command-line tool, not a service. "Deploying" means publishing a release that people install, plus
publishing the project site.

## Environments and promotion path
| Environment | Purpose | Promotion from | Approval |
| --- | --- | --- | --- |
| Pull request | CI on the merge result: `verify` on 3 OSes (stable Rust) plus an MSRV build, CodeQL, Gitleaks, Semgrep, dependency review | feature branch | the required checks green (`verify` on 3 OSes, `msrv`, `branch-name`, `codeql`, `secrets`, `pentest`), conversations resolved |
| `main` | Always releasable | pull request | squash or rebase merge (linear history), no direct pushes, applies to admins |
| GitHub Pages | Project site at `https://ifthenelse.github.io/migadu-cli/` | `main`, on demand | the maintainer runs the **Pages** workflow from the Actions tab (`pages.yml`) |
| GitHub Release | Downloadable archives, source package, checksums | `main`, on demand | the maintainer runs the **Release** workflow from the Actions tab (`release.yml`) |

There is no custom domain; the site lives at the GitHub Pages project URL, which is also the repository's homepage link.

## Branching policy
- `main` is the only long-lived branch and is protected: changes arrive by pull request only, the required checks (`verify` on 3 OSes, `msrv`, `branch-name`, `codeql`, `secrets`, `pentest`) must pass on a branch that is up to date with `main`, conversations must be resolved, history stays linear (squash merge is the only merge method), and force pushes and deletions are blocked, for administrators too.
- Work happens on short-lived branches named `<type>/<short-description>` (`feat/`, `fix/`, `docs/`, `ci/`, `chore/`, `refactor/`, `test/`, plus `dependabot/` and `alert-autofix-`, which the tools create). Branches are deleted automatically after merge. GitHub rulesets cannot enforce branch names on a personal repository, so the `branch-name` CI job does.
- Release tags `v*` are protected: they cannot be moved or deleted. Only the Release workflow creates them.

## Release
The version has one source: `version` in `Cargo.toml` (`Cargo.lock` follows it), and it follows [Semantic Versioning 2.0.0](https://semver.org/). Every pull request bumps it:
MAJOR for a backwards-incompatible change to commands, options, output or configuration, MINOR for backwards-compatible
functionality, PATCH for fixes, documentation and CI. While the major version is 0, bump MINOR for breaking changes.

1. Bump `version` in `Cargo.toml` in the pull request that makes the change (run `cargo build` so `Cargo.lock` follows), and merge it into `main`. Merging does not publish anything.
2. When you want to release, open **Actions → Release → Run workflow**, choose `main`, and optionally tick *draft* to review the release on
   GitHub before it goes public. The jobs:
   - **plan:** refuses to run on any branch other than `main`, reads the version, derives the tag `v<version>`, and stops quietly if a release with that tag already exists.
   - **verify:** runs `scripts/verify` (format, clippy, tests).
   - **build:** compiles natively on each operating system, in release mode with `--locked`: `darwin-arm64`, `darwin-x64`
     (cross-built on the Arm runner), `linux-x64`, `linux-arm64` and `windows-x64`. It smoke-tests every binary the runner can
     execute (prints help, never echoes a test secret) and archives each with `LICENSE` and `README.md` (`.tar.gz`, or `.zip`
     for Windows).
   - **publish:** adds the source package from `cargo package` and `SHA256SUMS`, then creates the tag on the commit and the GitHub release with generated notes. This is the only job with write permission.

Versions that were merged but never released simply have no tag. The next release carries their changes.

To publish the site, run **Actions → Pages → Run workflow** on `main`; it deploys the `site/` folder.

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
