# migadu-cli: instructions for AI coding agents

## Scope
- This file is the canonical contract for coding agents (Claude Code, Codex, Copilot, Aider).
  Tool adapters (`CLAUDE.md`, `.github/copilot-instructions.md`) only point here.
- Purpose and non-goals: `intent.md`. Architecture: `docs/architecture.md`. Decisions: `docs/adr/`.

## Stack
- Rust (edition 2021, `rust-version` 1.85). Two direct dependencies: `ureq` (HTTPS, rustls) and `serde_json`.
- Layout: `src/` (library and `main.rs`), `tests/` (offline tests with fakes), `docs/` (see `docs/architecture.md`).

## Commands
- Verify everything: `scripts/verify` (runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`).
- Build and run: `cargo run --release -- --help`.

## Verification
- Run `scripts/verify` before handing work back; it must pass. CI runs the same command and is the authoritative gate.
- Add or update tests for behaviour you change. Live or state-changing tests must be opt-in.

## Constraints
- Commit messages follow Conventional Commits (see `.gitmessage.txt`). Keep changes small and focused.
- Do not reformat unrelated code. Do not edit generated files or lockfiles by hand.
- Versioning follows [Semantic Versioning 2.0.0](https://semver.org/). The single version source is `version` in
  `Cargo.toml` (`Cargo.lock` follows it). **Every pull request bumps it**, and only for the change it contains:
  MAJOR for a backwards-incompatible change to commands, options, output or configuration; MINOR for backwards-compatible
  functionality; PATCH for fixes, documentation, CI and other changes that leave behaviour untouched. While the major
  version is 0, anything may change, but still bump MINOR for breaking changes and PATCH otherwise.
  Release tags are `vMAJOR.MINOR.PATCH` and must equal that field (the release workflow enforces it).
- Never commit secrets or `.env` files. `.env.example` holds names and safe placeholders only.

## Approval boundaries
Ask the user before: adding or upgrading a dependency, changing CI/CD or infrastructure, changing a data
schema or public contract, deleting files, touching secrets or credentials, pushing or opening a PR.

## Hand-off
Summarise what changed and why, which checks you ran with their result, and any open risk or follow-up.
