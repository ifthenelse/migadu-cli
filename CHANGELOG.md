# Changelog

User-visible and operationally material changes. Release notes with every commit are generated on each
[GitHub release](https://github.com/ifthenelse/migadu-cli/releases). Format: [Keep a Changelog](https://keepachangelog.com),
[SemVer](https://semver.org).

## [Unreleased]

## [0.4.3] - 2026-11-28
### Added
- Project site: interactive ASCII-art background (Migadu mail tokens, envelopes and an Alpine skyline that react to the pointer; still frame with reduced motion).

## [0.4.2] - 2026-11-28
### Changed
- Releases and the Pages site are published on demand from the Actions tab instead of automatically; the release workflow can create a draft.
- Branching policy documented; release tags are protected.

## [0.4.1] - 2026-11-28
### Changed
- Ignore stray build artifacts and private promotional files; track the shared Codex MCP config.

## [0.4.0] - 2026-11-24
### Added
- `migadu --version` (and `-V`) prints the version.

## [0.3.4] - 2026-11-24
### Added
- Demo GIF in the README, with its recording script in `docs/demo.tape`.

## [0.3.3] - 2026-11-24
### Changed
- New logo and header in light and dark variants for the README and the project site.

## [0.3.2] - 2026-11-24
### Changed
- Documentation refreshed after the Rust rewrite; leftover Node.js files removed.

## [0.3.1] - 2026-11-24
### Added
- Install instructions per operating system with checksum verification.

## [0.3.0] - 2026-11-24
### Changed
- **Breaking:** rewritten in Rust. The Node.js and Bun builds are gone; releases ship native binaries of about 1.4 MB for
  macOS (Arm and Intel), Linux (x64 and Arm) and Windows (x64), plus a source package and `SHA256SUMS`.
  Commands, options, output and exit codes are unchanged.

## [0.2.3] - 2026-11-24
First published release (Node.js; versions before it were never tagged).
### Added
- Domains, mailboxes, aliases, rewrites, identities and forwardings through the Migadu REST API, the `api` pass-through,
  and API-key lookup from 1Password, Bitwarden, pass, KeePassXC and the OS keychain.
- `migadu tier`: plan and limits read from the admin panel with a session cookie.
- `migadu catchall`, `migadu spam` and `migadu bcc`, driven through the admin panel's forms.
- Releases are tagged, built and published automatically when the version changes on `main`.

[Unreleased]: https://github.com/ifthenelse/migadu-cli/compare/v0.4.3...HEAD
[0.4.3]: https://github.com/ifthenelse/migadu-cli/compare/v0.4.2...v0.4.3
[0.4.2]: https://github.com/ifthenelse/migadu-cli/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/ifthenelse/migadu-cli/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/ifthenelse/migadu-cli/compare/v0.3.4...v0.4.0
[0.3.4]: https://github.com/ifthenelse/migadu-cli/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/ifthenelse/migadu-cli/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/ifthenelse/migadu-cli/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/ifthenelse/migadu-cli/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/ifthenelse/migadu-cli/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/ifthenelse/migadu-cli/releases/tag/v0.2.3
