# 0005. Stand-alone binaries built with Bun

- Status: superseded by [0006](0006-rewrite-in-rust.md)
- Date: 2026-11-24

## Context
The Node-based install needs Node 22 or later, which is a barrier for people who just want a command. A rewrite in
another language (Rust was considered) would mean re-implementing and re-testing about 500 lines of tested behaviour,
including panel quirks, and keeping two codebases in step while the panel work is still moving.

## Decision
Keep the JavaScript source as the single implementation and publish stand-alone binaries compiled with
`bun build --compile` for darwin-arm64, darwin-x64, linux-x64, linux-arm64 and windows-x64. Pin the Bun version. Build
only from a version-matching tag, publish SHA-256 checksums, and smoke-test the Linux binary.

## Alternatives considered
- Rewrite in Rust or Go: smaller, faster binaries, but a rewrite and a second code path for little user benefit at this size.
- Node single-executable applications (`--build-sea`): built a binary, but the macOS result was killed until re-signed, and it needed a separate bundling step for the multi-file source.
- Publishing to npm only: still requires Node.

## Consequences
One code path and an install without Node. Binaries are 87–115 MB because they embed the runtime, and macOS binaries
are not notarised. Bun becomes a build-time tool and the binary runs on Bun's runtime; the Node install remains the
reference. Revisit if binary size or Bun compatibility becomes a problem.
