# Architecture

One canonical location for the current architecture. Business intent is in [`intent.md`](../intent.md); decisions are
in [`adr/`](adr/); behaviour is in the [functional specification](specifications/functional-spec.md).

## System context
`migadu` is a command-line program run by one person on their own machine (or in CI). It talks to two Migadu
endpoints and, optionally, to a local password-manager program:

```
                     ┌────────────────────────────┐
  user / script ───► │  migadu (Node.js, ESM)     │
                     │  bin/migadu.js → src/cli.js│
                     └─────┬──────────┬───────────┘
                           │          │ spawn, no shell
        HTTPS Basic auth   │          └──► op · bw · pass · keepassxc-cli · security · secret-tool
        email:apikey       ▼
   api.migadu.com/v1  (REST)          HTTPS + session cookie
                                      ▼
                           admin.migadu.com (server-rendered HTML panel)
```

Neighbouring systems: Migadu's REST API (official, early beta), Migadu's admin panel (no API; the CLI replays the
same HTML forms a browser would submit), and the user's password manager CLI.

## Components and boundaries
| File | Responsibility |
| --- | --- |
| `bin/migadu.js` | Entry point. Calls `main(process.argv.slice(2))` and sets the exit code. |
| `src/cli.js` | Argument parsing (`util.parseArgs`), resource table, command dispatch, output (JSON or table), help text. The only place that prints. |
| `src/api.js` | `createClient`: Basic-auth REST client with a 30 s timeout. Normalises errors into `MigaduError`. Treats a `DELETE` that returns HTTP 500 followed by a 404 on re-read as success. |
| `src/secrets.js` | `resolveSecret` maps a reference (`op://…`, `bw:`, `pass:`, `keepassxc:`, `keychain:`) to a vendor CLI call. `run` spawns it without a shell. `loadCredentials` reads the environment. |
| `src/web.js` | Admin-panel client: plan, catchall, spam settings and Bcc captures, through the session cookie. Verifies each write by re-reading the page. |
| `src/forms.js` | Small HTML form reader. Returns what a browser would submit for a form, so a write can resend untouched fields. Strips HTML comments first. |
| `test/` | Offline tests with mocked `fetch` and fake panel pages. |

Dependency direction: `bin → cli → {api, secrets, web}`, and `web → forms`. There are no third-party dependencies.
Everything with side effects (`fetchImpl`, `exec`, `out`, `err`, `stdin`, `env`) is injected into `main`, which is
what makes the CLI testable without network or processes.

## Data flows and integration contracts
1. **REST command** (`migadu mailboxes list -d example.org`): resolve credentials, build `GET /domains/example.org/mailboxes`,
   send with `Authorization: Basic base64(email:apikey)`, print the JSON (or a table).
2. **Credential lookup:** `MIGADU_API_KEY` wins. Otherwise `MIGADU_API_KEY_REF` is resolved by spawning the vendor CLI;
   the first line of stdout is the secret. Nothing is written to disk.
3. **Panel read or write** (`catchall`, `spam`, `bcc`, `tier`): the user supplies `MIGADU_WEB_COOKIE` (or a `_REF` to it).
   The CLI fetches the HTML page, reads the form with its CSRF token, changes only the requested fields, POSTs it
   (using `_method=put` or `delete` as the panel does), then re-reads the page and fails loudly if the change did not apply.
4. **Raw call** (`migadu api GET /domains`): any method and path on the REST API, for endpoints the CLI does not wrap.

Contracts: the REST API is Migadu's own (`https://api.migadu.com/v1`). The panel contract is observed, not published;
the mapped forms are in [`web-panel.md`](web-panel.md).

## Trust boundaries
The API key and the session cookie are the sensitive values. They live in the environment or a password manager, are
sent only to Migadu over HTTPS, and are never printed or logged. The panel client is the fragile edge: it depends on
markup Migadu can change. See the [threat model](security/threat-model.md).

## Quality attributes
- **Portability:** Node 22 or later; no native modules; tested on Ubuntu, macOS and Windows with Node 22 and 24.
- **Safety:** deletes need `--yes`; panel writes are verified; unknown spam keys are rejected.
- **Size:** about 500 lines of source, so the whole program can be read in an afternoon.
