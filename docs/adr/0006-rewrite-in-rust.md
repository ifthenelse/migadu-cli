# 0006. Rewrite in Rust and ship native binaries

- Status: accepted
- Date: 2026-11-24
- Supersedes: [0002](0002-zero-runtime-dependencies.md), [0005](0005-standalone-binaries-with-bun.md)

## Context
The JavaScript version needed Node 22 or later, and the stand-alone binaries built from it with Bun were 87–115 MB
because they embedded a runtime. The owner asked for a real executable and chose a rewrite in Rust. The program was
about 500 lines with 22 tests, which made a faithful port practical.

## Decision
Rewrite the CLI in Rust and delete the JavaScript implementation. Keep behaviour and output the same, and port the tests.
Use two direct dependencies: `ureq` (blocking HTTPS with rustls, no async runtime, no OpenSSL) and `serde_json` (with
`preserve_order` so JSON output keeps its key order). Hand-write argument parsing, base64 and the small HTML scanner.
The edition is 2021 with `rust-version = 1.85`, checked in CI. Release builds use `opt-level = "z"`, LTO and `strip`.
Native binaries are built on their own operating systems in the release workflow, for darwin-arm64, darwin-x64,
linux-x64, linux-arm64 and windows-x64.

## Alternatives considered
- Keep JavaScript with Bun binaries: one code path, but 87–115 MB executables that embed a second runtime (decision 0005).
- `clap` for arguments and a regex crate for HTML: faster to write, but more crates for a small program. Dropped to keep
  the dependency tree at about 38 crates.
- Go: similar result, but the owner preferred Rust.

## Consequences
Binaries are about 1.4 MB, start instantly and need no runtime. The project now has third-party dependencies to watch
(Dependabot covers Cargo), and the zero-dependency principle of 0002 is replaced by "as few as practical". Node is no
longer needed to run or contribute. The panel HTML scanner is hand-written and tied to current markup, as before.
