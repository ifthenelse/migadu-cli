# Admin panel (web) notes

Mapped read-only on 2026-11-24 from `admin.migadu.com` in a signed-in browser. The panel is server-rendered
HTML: no XHR/JSON calls were observed on the dashboard, billing, domain or mailbox pages. Writes are HTML forms
that POST with a `_csrf_token` hidden field. Domains are addressed by numeric id (`/domains/<id>`), not name.

| Area | Path | Form (POST) fields |
| --- | --- | --- |
| Plan | `/billing/subscription` (also the header link, e.g. "Mini / monthly") | read-only |
| Domain list | `/domains` | read-only |
| New mailbox | `/domains/<id>/mailboxes` | `mailbox[local_part]`, `mailbox[name]`, `mailbox[password_method]`, `mailbox[password_recovery_email]`, `mailbox[tags]` |
| New alias | `/domains/<id>/aliases` | `alias[local_part]`, `alias[destinations]`, `alias[tags]` |
| Catchall | `PUT /domains/<id>` (POST with `_method=put`, `template=catchall`, `origin_path`) | `domain[catchall_destinations]`, newline or comma separated; read from `/domains/<id>/catchall` |
| Listing settings | `/domains/<id>` | `description`, `tags` |
| Read-only pages | `activity`, `aliasing`, `rewrites`, `bccs`, `spam`, `dns`, `notes`, `transfer`, `deactivation`, `delete` | not mapped |

Implemented in the CLI: reading the plan (`migadu tier`) and reading/writing the catchall (`migadu catchall`), both with `MIGADU_WEB_COOKIE`. Other web writes are not
implemented; the REST API covers mailboxes, aliases and rewrites. Catchall, Bcc captures, spam filtering and
domain aliases are not confirmed to exist in the REST API.
