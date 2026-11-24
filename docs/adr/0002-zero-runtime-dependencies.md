# 0002. Zero runtime dependencies

- Status: superseded by [0006](0006-rewrite-in-rust.md)
- Date: 2026-11-24

## Context
The tool handles an API key and a session cookie and is meant to be installed by people who read what they run. Every
dependency widens the supply-chain surface and complicates a cross-platform install.

## Decision
Use only the Node.js standard library: global `fetch`, `util.parseArgs`, `node:child_process`, `node:test`. Require Node 22 or later. Parse the few HTML forms needed with a small purpose-built reader instead of an HTML library.

## Alternatives considered
- A CLI framework (commander, yargs): saves little code for about 12 commands.
- An HTML parser (cheerio, parse5): more robust against markup changes, but adds a dependency tree for four forms.
- An HTTP library (axios, undici): `fetch` is built in.

## Consequences
Smaller attack surface and a source small enough to audit. The panel parser is regex-based and may break when Migadu changes its markup; if that happens often, revisit with an ADR that supersedes this one.
