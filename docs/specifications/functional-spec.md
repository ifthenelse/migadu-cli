# Functional specification

What `migadu` does, as observed from the command line. Version 0.3.x. For internals see
[architecture](../architecture.md); for the panel forms see [web-panel](../web-panel.md).

## Invocation
```
migadu [options] <resource> <action> [args]
```
Exit codes: `0` success, `1` any failure while running a command (API, panel, credentials, missing arguments, refusing a delete),
`2` unparseable options or no command at all.
Errors are one line on stderr, `error: <message>`. Output on stdout is JSON by default.

## Global options
| Option | Meaning |
| --- | --- |
| `-d, --domain <name>` | Domain to act on. Falls back to `MIGADU_DOMAIN`. |
| `-f, --field k=v` | Body field, repeatable. `k:=<json>` sends a number, boolean, array or object. |
| `--json <obj\|@file\|->` | Full JSON body: inline, from a file, or from stdin. `-f` fields are merged on top. |
| `-y, --yes` | Confirms a delete or a clearing action. Without it the command refuses. |
| `-o, --output json\|table` | Output format. `table` shows up to six scalar columns. |
| `-h, --help` | Print usage. |
| `-V, --version` | Print `migadu <version>` and exit; needs no credentials. |

## Configuration
| Variable | Required | Meaning |
| --- | --- | --- |
| `MIGADU_EMAIL` | for REST commands | Account email used for Basic auth. |
| `MIGADU_API_KEY` | one of these two | API key in the environment. |
| `MIGADU_API_KEY_REF` | one of these two | Password-manager reference for the API key. |
| `MIGADU_DOMAIN` | no | Default domain for `-d`. |
| `MIGADU_WEB_COOKIE` | for panel commands | The `Cookie` header from a signed-in admin.migadu.com session. |
| `MIGADU_WEB_COOKIE_REF` | alternative | Password-manager reference for that cookie. |

Reference forms: `op://vault/item/field` (1Password), `bw:<item>` (Bitwarden), `pass:<name>`,
`keepassxc:<file.kdbx>#<entry>`, `keychain:<service>` (macOS `security`, Linux `secret-tool`; not on Windows).
The secret is the first line of the vendor CLI's output. An unrecognised reference fails with a message that does not repeat it.

## REST resources
| Resource | Scope | Actions | Endpoint pattern |
| --- | --- | --- | --- |
| `mailboxes` | domain | list, get, create, update, delete | `/domains/{d}/mailboxes[/{id}]` |
| `aliases` | domain | list, get, create, update, delete | `/domains/{d}/aliases[/{id}]` |
| `rewrites` | domain | list, get, create, update, delete | `/domains/{d}/rewrites[/{id}]` |
| `identities` | mailbox | list, get, create, update, delete | `/domains/{d}/mailboxes/{mailbox}/identities[/{id}]` |
| `forwardings` | mailbox | list, get, create, update, delete | `/domains/{d}/mailboxes/{mailbox}/forwardings[/{id}]` |
| `domains` | account | list, get, create, update, records, diagnostics, usage, activate | `/domains[/{name}[/records\|diagnostics\|usage\|activate]]` |
| `api <METHOD> <path>` | any | any | passes through to `https://api.migadu.com/v1` |

For `identities` and `forwardings` the mailbox is the first positional argument, then the id.
Responses are printed as the API returns them. With `-o table`, the first array found in the response is shown.

Examples:
```sh
migadu mailboxes create -d example.org -f local_part=info -f name="Info" -f password=… 
migadu mailboxes update -d example.org info -f may_send:=false
migadu forwardings create -d example.org info -f address=me@example.net
migadu mailboxes delete -d example.org info --yes
```

### Delete behaviour
`delete` without `--yes` exits `1` with `refusing to delete … without --yes` and changes nothing. Migadu sometimes answers a successful delete with HTTP 500; the
client then re-reads the item and treats a 404 as success.

## Panel commands (need `MIGADU_WEB_COOKIE`)
### `tier`
Reads the plan name and billing period from the admin panel header, the renewal date, and the plan limits from a built-in
table (messages per day, storage, API access). With the API credentials also set, it adds per-domain usage. Works on the
cookie alone. Without a cookie it explains how to supply one.

### `catchall get|set [domain]`
`get` lists the catchall destinations. `set` takes `-f destinations=a@x.org,b@x.org`; an empty list clears the catchall and needs `--yes`,
posts the panel's form, re-reads it, and returns the previous and new values. The panel may rewrite recipients onto the
domain (`user@other.tld` becomes `user@domain`), so the check compares local parts and adds a note when this happens.
If the panel did not apply the change, the command fails with `not applied`.

### `spam get|set [domain]`
`get` reads four panel forms. `set` changes only the keys given and resends every other field of the same form unchanged.

| Key | Values |
| --- | --- |
| `aggressiveness` | `3` permissive, `2` suspicious, `0` moderate (default), `-2` aggressive, `-3` paranoid |
| `subject_rewriting` | true or false |
| `junk_subject_keyword` | text |
| `sender_denylist`, `sender_junklist`, `sender_allowlist`, `recipient_denylist` | newline or comma separated lists |
| `allowlist_precedence` | true or false |

An unknown key fails with `unknown spam setting`. Changes the panel did not apply are reported.

### `bcc list|create|update|delete`
Blind-copy captures per domain. The domain comes from `-d` or `MIGADU_DOMAIN`. `create` needs `-f address=… -f destination=…` and optionally
`-f direction=incoming|outgoing|all`. `update <address>` takes the same fields. `delete <address>` needs `--yes` and fills in
the address confirmation the panel requires. Create and delete are re-read to confirm.

## Not implemented
Domain aliases (the create form is mapped; list and delete markup is not verified), domain notes, listing settings, and any
panel write other than catchall, spam and Bcc.

## Failure cases
| Situation | Result |
| --- | --- |
| No `MIGADU_EMAIL` or API key | `error: MIGADU_EMAIL is not set` or the API-key message, exit 1 |
| Panel session expired | `error: web session expired: sign in at admin.migadu.com and copy a fresh cookie`, exit 1 |
| Panel markup changed | `could not find …` style errors; nothing is written |
| HTTP error from the API | `error: METHOD /path -> HTTP status: message`, exit 1 |
| Password manager CLI not installed | `error: <cmd> not found on PATH`, exit 1 |
| Unknown option or no command | parse error or usage text, exit 2 |
| Missing id, domain or mailbox | `error: usage: …` or `error: domain required: use --domain or MIGADU_DOMAIN`, exit 1 |
