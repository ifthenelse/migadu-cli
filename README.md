<p align="center">
  <img src=".github/assets/banner.svg" alt="migadu-cli: run your Migadu email hosting from the terminal" width="100%">
</p>

# migadu-cli

Manage your [Migadu](https://www.migadu.com) email hosting from the terminal on Windows, macOS and Linux, with API keys pulled straight from your password manager.

[![CI](https://github.com/ifthenelse/migadu-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/ifthenelse/migadu-cli/actions/workflows/ci.yml)
[![Security](https://github.com/ifthenelse/migadu-cli/actions/workflows/security.yml/badge.svg)](https://github.com/ifthenelse/migadu-cli/actions/workflows/security.yml)
[![License](https://img.shields.io/github/license/ifthenelse/migadu-cli)](LICENSE)

Migadu's admin panel is great for one mailbox and slow for forty. `migadu` wraps the Migadu REST API so domains, mailboxes, aliases, identities, forwardings and rewrites are scriptable, and the API key never has to sit in a dotfile. It has zero npm dependencies and is not an official Migadu product.

## Features

- **Whole API surface** — domains (records, diagnostics, usage, activate), mailboxes, aliases, rewrites, identities and forwardings, plus `migadu api <METHOD> <path>` for any other endpoint.
- **Password-manager credentials** — 1Password, Bitwarden, `pass`, KeePassXC, macOS Keychain and Linux Secret Service, read by calling each tool's CLI without a shell.
- **Safe by default** — deletes need `--yes`, and the CLI confirms a delete that Migadu reports as HTTP 500 but actually performed.
- **Tier report** — `migadu tier` prints the plan/usage data the API returns for each domain.
- **Cross-platform, zero dependencies** — plain Node.js, tested on Linux, macOS and Windows in CI.

## Quickstart

Requires Node.js 22 or newer and a [Migadu API key](https://admin.migadu.com/account/api/keys).

```sh
git clone https://github.com/ifthenelse/migadu-cli.git
cd migadu-cli
export MIGADU_EMAIL=<your-admin-email>
export MIGADU_API_KEY=<your-api-key>
node bin/migadu.js domains list
```

You should see the JSON list of domains on your account. Add `"bin"` to your `PATH`, or run `npm link` inside the clone, to call it as `migadu`.

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

## Limitations

- The Migadu API documents no plan or tier endpoint. `migadu tier` shows any plan/tier field the API returns and your usage, and otherwise tells you to check the billing page.
- The endpoint paths follow Migadu's public API and are covered by mocked tests only; they have not yet been run against a live account.

## Development

```sh
npm test
npm run lint
```

## Contributing

Issues and pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
