<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/banner-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img alt="migadu-cli: the maintained command line for Migadu email hosting" src=".github/assets/banner-light.svg" width="100%">
  </picture>
</p>

# migadu-cli

The maintained command line for [Migadu](https://www.migadu.com) email hosting: new project, new mailboxes, done in the terminal on Windows, macOS and Linux, with your API key pulled from your password manager and never stored in a file.

[![CI](https://github.com/ifthenelse/migadu-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/ifthenelse/migadu-cli/actions/workflows/ci.yml)
[![Security](https://github.com/ifthenelse/migadu-cli/actions/workflows/security.yml/badge.svg)](https://github.com/ifthenelse/migadu-cli/actions/workflows/security.yml)
[![Release](https://img.shields.io/github/v/release/ifthenelse/migadu-cli)](https://github.com/ifthenelse/migadu-cli/releases/latest)
[![License](https://img.shields.io/github/license/ifthenelse/migadu-cli)](LICENSE)

[Install](#install) · [Quickstart](#quickstart) · [Usage](#usage) · [Password managers](#password-managers) · [Configuration](#configuration) · [Contributing](#contributing)

Migadu's admin panel is great for one mailbox and slow for forty. `migadu` wraps the Migadu REST API so domains, mailboxes, aliases, identities, forwardings and rewrites are scriptable, and the API key never has to sit in a dotfile. It ships as one small native binary (about 1.4 MB, written in Rust) and is not an official Migadu product.

## Features

- **Whole API surface** — domains (records, diagnostics, usage, activate), mailboxes, aliases, rewrites, identities and forwardings, plus `migadu api <METHOD> <path>` for any other endpoint.
- **Password-manager credentials** — 1Password, Bitwarden, `pass`, KeePassXC, macOS Keychain and Linux Secret Service, read by calling each tool's CLI without a shell.
- **Safe by default** — deletes need `--yes`, and the CLI confirms a delete that Migadu reports as HTTP 500 but actually performed.
- **Tier report** — `migadu tier` shows your plan, renewal date and plan limits (read from the admin panel with a session cookie) plus per-domain API usage.
- **One native binary** — no runtime to install; built and tested on Linux, macOS and Windows in CI, with two direct dependencies (an HTTPS client and a JSON library).

## Install

Pick the archive for your system from the [latest release](https://github.com/ifthenelse/migadu-cli/releases/latest).
Each one holds the `migadu` binary, `LICENSE` and `README.md`.

| System | Archive |
| --- | --- |
| macOS, Apple silicon | `migadu-<version>-darwin-arm64.tar.gz` |
| macOS, Intel | `migadu-<version>-darwin-x64.tar.gz` |
| Linux, x86-64 | `migadu-<version>-linux-x64.tar.gz` |
| Linux, Arm64 | `migadu-<version>-linux-arm64.tar.gz` |
| Windows, x86-64 | `migadu-<version>-windows-x64.zip` |

macOS and Linux (replace the version and the system name from the table):

```sh
VERSION=v0.3.0
SYSTEM=darwin-arm64
curl -fLO "https://github.com/ifthenelse/migadu-cli/releases/download/$VERSION/migadu-$VERSION-$SYSTEM.tar.gz"
curl -fLO "https://github.com/ifthenelse/migadu-cli/releases/download/$VERSION/SHA256SUMS"
shasum -a 256 -c SHA256SUMS --ignore-missing   # on Linux: sha256sum -c SHA256SUMS --ignore-missing
tar -xzf "migadu-$VERSION-$SYSTEM.tar.gz"
sudo install "migadu-$VERSION-$SYSTEM/migadu" /usr/local/bin/migadu
migadu --help
```

If you downloaded the archive with a browser, macOS may block the unsigned binary; clear the flag with
`xattr -d com.apple.quarantine /usr/local/bin/migadu`.

Windows (PowerShell): download the `.zip`, check it with `Get-FileHash <file> -Algorithm SHA256` against `SHA256SUMS`,
extract it, and put the folder with `migadu.exe` on your `PATH`.

From source, with [Rust](https://rustup.rs) 1.85 or newer: `cargo install --git https://github.com/ifthenelse/migadu-cli`.

## Quickstart

You need a [Migadu API key](https://admin.migadu.com/account/api/keys) (the API is not available on the Micro plan).

```sh
export MIGADU_EMAIL=<your-admin-email>
export MIGADU_API_KEY=<your-api-key>
migadu domains list
```

You should see the JSON list of domains on your account. To keep the key out of your shell history and dotfiles, point
`MIGADU_API_KEY_REF` at your password manager instead, for example `op://Private/Migadu/credential`
(see [Password managers](#password-managers)).

## Usage

```sh
# Plan and usage information
migadu tier

# Mailboxes (typed values use :=, plain strings use =)
migadu mailboxes list -d example.org
migadu mailboxes create -d example.org -f local_part=alice -f name=Alice -f password=<secret> -f may_send:=true
migadu mailboxes delete alice -d example.org --yes

# Onboard a domain
migadu domains create -f name=example.org
migadu domains records example.org
migadu domains diagnostics example.org
migadu domains activate example.org

# Catchall recipients (admin panel; needs MIGADU_WEB_COOKIE)
migadu catchall get example.org
migadu catchall set example.org -f destinations=alice@example.org,bob@example.org

# Spam filtering (admin panel). aggressiveness: 3 permissive, 2 suspicious, 0 moderate, -2 aggressive, -3 paranoid
migadu spam get example.org
migadu spam set example.org -f aggressiveness=-2 -f subject_rewriting:=true -f sender_denylist=spam@bad.example

# Bcc captures (admin panel)
migadu bcc create -d example.org -f address=audit -f destination=archive -f direction=incoming
migadu bcc list -d example.org
migadu bcc delete audit -d example.org --yes

# Anything else
migadu api GET /domains/example.org/rewrites -o table
```

Run `migadu --help` for every resource and action. Bodies can also come from `--json '{...}'`, `--json @file.json` or `--json -` (stdin).

## Password managers

Set `MIGADU_API_KEY_REF` instead of `MIGADU_API_KEY`:

| Manager | Reference | Runs |
| --- | --- | --- |
| 1Password | `op://Private/Migadu/credential` | `op read` |
| Bitwarden | `bw:<item name or id>` | `bw get password` |
| pass | `pass:mail/migadu` | `pass show` |
| KeePassXC | `keepassxc:/path/vault.kdbx#Migadu` | `keepassxc-cli show -s -a Password` |
| macOS Keychain | `keychain:<service>` | `security find-generic-password -s <service> -w` |
| Linux Secret Service | `keychain:<service>` | `secret-tool lookup service <service>` |

Windows Credential Manager has no built-in read CLI, so on Windows use 1Password, Bitwarden or KeePassXC.

## Configuration

| Variable | Default | Description |
| --- | --- | --- |
| `MIGADU_EMAIL` | none, required | Admin email that owns the API key |
| `MIGADU_API_KEY` | none | API key in plain form |
| `MIGADU_API_KEY_REF` | none | Password-manager reference, used when `MIGADU_API_KEY` is unset |
| `MIGADU_DOMAIN` | none | Default domain, same as `-d` |
| `MIGADU_WEB_COOKIE` | none | `Cookie` header copied from a signed-in admin.migadu.com tab; enables the plan lookup and the panel-based commands (`tier`, `catchall`, `spam`, `bcc`) |
| `MIGADU_WEB_COOKIE_REF` | none | Same, read from a password manager (references as above) |

## Limitations

- The REST API has no plan endpoint, so `migadu tier` reads the plan from the admin panel's HTML with a cookie you supply. It is read-only and breaks if Migadu changes the page markup. The plan limits table is a snapshot from 2026-11-24.
- `migadu catchall` drives the admin panel's HTML form with your session cookie. It re-reads the page after saving and fails if the change did not stick. The panel stores recipients as addresses on the same domain (`admin@other.com` becomes `admin@example.org`), and the output says when it did that. The Node.js version of this feature was verified with a live write and restore on a test domain.
- `migadu spam` and `migadu bcc` also drive the panel's forms, with the same live verification on the Node.js version. Domain aliases are mapped in [docs/web-panel.md](docs/web-panel.md) but not implemented.
- Since the rewrite in Rust (v0.3.0), the panel features are covered by tests against fake panel pages; the live check has not been repeated with the Rust binary. The REST endpoint paths follow Migadu's public API and are covered by mocked tests.

## Development

```sh
cargo test          # offline tests with fake HTTP and password managers
bash scripts/verify # fmt, clippy and tests: the same command CI runs
```

## Contributing

Issues and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
