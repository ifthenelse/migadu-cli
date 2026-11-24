# 0003. REST API first, optional admin-panel mode

- Status: accepted
- Date: 2026-11-24

## Context
Migadu's REST API (documented as early beta) covers mailboxes, aliases, rewrites, identities, forwardings and domains.
It does not return the plan name and does not expose catchall, spam filtering or Bcc captures. The admin panel is
server-rendered HTML with CSRF-protected forms and no JSON endpoints.

## Decision
Build every supported feature on the REST API. For the gaps, add an opt-in panel mode: the user supplies a session
cookie (`MIGADU_WEB_COOKIE` or `_REF`), and the CLI replays the panel's own forms. The CLI never asks for or stores a
password. Every panel write is re-read and fails loudly if it did not apply.

## Alternatives considered
- API only: leaves plan detection and several settings out, which the product promise needs.
- Logging in with a password: the CLI would handle a password and 2FA; rejected.
- A headless browser: heavy, and a large new dependency.

## Consequences
Users get complete coverage without the panel in the common case. Panel features are best-effort: they can break
without notice, and automation of the panel may be restricted by Migadu's terms. See the [runbook](../runbooks/panel-markup-changed.md).
