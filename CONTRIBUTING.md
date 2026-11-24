# Contributing

Applies to humans and coding agents alike.

## Workflow
1. Branch from `main` (or use a git worktree per task; never share one working copy between two agents).
2. Make a focused change; keep unrelated refactors out.
3. Run `scripts/verify` locally (needs [Rust](https://rustup.rs) 1.85 or newer). CI runs the same command on Linux, macOS and Windows and is the authoritative gate.
4. Open a pull request: shared and production code enters only through a PR with required checks.

## Demo GIF
The README GIF is recorded from `docs/demo.steps` and `docs/demo.tape` with [VHS](https://github.com/charmbracelet/vhs): build the release binary, put it on your `PATH` (`export PATH="$PWD/target/release:$PATH"`), then run `vhs docs/demo.tape`. It only uses commands that need no account (a fake key gets a real 401 from Migadu). Re-record it when messages or output change.

## Commit convention
Conventional Commits, see `.gitmessage.txt`: `type(scope)!: description`.
Enable the template once: `git config commit.template .gitmessage.txt`.

## Pull request requirements
- Description of the change and its reason; link to the related decision or ticket.
- `scripts/verify` green; tests added or updated for changed behaviour.
- Docs, ADR or changelog updated when behaviour, contracts or architecture change.
- `version` in `Cargo.toml` bumped following [semver 2.0.0](https://semver.org/) (see `AGENTS.md`); merging a version change publishes a release.
- Dependency changes reviewed explicitly (new crates, version bumps, `Cargo.lock` diff).

## Review roles
- Reviewer: the maintainer ([CODEOWNERS](.github/CODEOWNERS)). Copilot's automatic review comments must be resolved before merging.
- Single-maintainer repository: self-review uses the checklist in the PR template, and all required checks must pass; branch protection applies to admins too.

## Escalation
Vulnerabilities: report privately as described in [SECURITY.md](SECURITY.md). Anything else: open an issue. If the release workflow or Pages deployment fails, see [docs/deployment.md](docs/deployment.md).
