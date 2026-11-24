<p align="center">
  <img src=".github/assets/banner.svg" alt="migadu-cli: run your Migadu email hosting from the terminal" width="100%">
</p>

# migadu-cli

Manage your [Migadu](https://www.migadu.com) email hosting from the terminal on Windows, macOS and Linux, with API keys pulled straight from your password manager.

[![CI](https://github.com/ifthenelse/migadu-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/ifthenelse/migadu-cli/actions/workflows/ci.yml)
[![Security](https://github.com/ifthenelse/migadu-cli/actions/workflows/security.yml/badge.svg)](https://github.com/ifthenelse/migadu-cli/actions/workflows/security.yml)
[![License](https://img.shields.io/github/license/ifthenelse/migadu-cli)](LICENSE)

Migadu's admin panel is great for one mailbox and slow for forty. `migadu` wraps the Migadu REST API so domains, mailboxes, aliases, identities, forwardings and rewrites are scriptable, and the API key never has to sit in a dotfile. It ships as one small native binary (about 1.4 MB, written in Rust) and is not an official Migadu product.

## Features

- **Whole API surface** — domains (records, diagnostics, usage, activate), mailboxes, aliases, rewrites, identities and forwardings, plus `migadu api <METHOD> <path>` for any other endpoint.
- **Password-manager credentials** — 1Password, Bitwarden, `pass`, KeePassXC, macOS Keychain and Linux Secret Service, read by calling each tool's CLI without a shell.
- **Safe by default** — deletes need `--yes`, and the CLI confirms a delete that Migadu reports as HTTP 500 but actually performed.
- **Tier report** — `migadu tier` shows your plan, renewal date and plan limits (read from the admin panel with a session cookie) plus per-domain API usage.
- **One native binary** — no runtime to install; built and tested on Linux, macOS and Windows in CI, with two direct dependencies (an HTTPS client and a JSON library).

## Quickstart

You need a [Migadu API key](https://admin.migadu.com/account/api/keys). Download the archive for your system from the
[latest release](https://github.com/ifthenelse/migadu-cli/releases/latest) (macOS, Linux and Windows, Intel/AMD and Arm),
unpack it, and put `migadu` on your `PATH`. Check it against `SHA256SUMS` from the same release.

```sh
export MIGADU_EMAIL=<your-admin-email>
export MIGADU_API_KEY=<your-api-key>
migadu domains list
```

You should see the JSON list of domains on your account. To build from source instead, install [Rust](https://rustup.rs) 1.85
or newer and run `cargo install --git https://github.com/ifthenelse/migadu-cli`.

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
- `migadu catchall` drives the admin panel's HTML form with your session cookie. It re-reads the page after saving and fails if the change did not stick. The panel stores recipients as addresses on the same domain (`admin@other.com` becomes `admin@example.org`), and the output says when it did that. Verified with a live write and restore on a test domain.
- `migadu spam` and `migadu bcc` also drive the panel's forms. Both were verified with a live write and restore on a test domain. Domain aliases are mapped in [docs/web-panel.md](docs/web-panel.md) but not implemented.
- The endpoint paths follow Migadu's public API and are covered by mocked tests only; they have not yet been run against a live account.

## Development

```sh
cargo test          # offline tests with fake HTTP and password managers
bash scripts/verify # fmt, clippy and tests: the same command CI runs
```

## Contributing

Issues and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
