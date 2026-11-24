# Testing

## Strategy
Everything is tested offline. `main` receives its collaborators as arguments (`fetchImpl`, `exec`, `out`, `err`,
`stdin`, `env`), so tests run the real CLI against a fake `fetch` and fake vendor commands. No test needs a network,
a password manager or a Migadu account.

Covered:
- **Command mapping:** each resource and action produces the right method, path and body, with Basic auth.
- **Safety rules:** deletes need `--yes`; clearing the catchall needs `--yes`; unknown spam keys are rejected.
- **API quirks:** a `DELETE` that returns 500 and then 404 on re-read counts as success.
- **Credentials:** every password-manager reference maps to the right command; a missing credential gives a clear
  message; an unsupported reference is never echoed back (regression test for a clear-text logging alert).
- **Panel behaviour:** form parsing returns what a browser would submit, ignores controls inside HTML comments, and
  preserves sibling fields on spam writes; catchall, spam and Bcc writes are verified by re-reading; an expired session
  gives a clear error.

Not covered by automated tests: the real Migadu API and the real admin panel. Those were checked by hand against a
throwaway domain, restoring the original values afterwards. Live tests must stay opt-in and isolated; there is no live
suite in CI.

## Suites
| Suite | Location | Speed | Run with |
| --- | --- | --- | --- |
| CLI, secrets, API client | `test/cli.test.js` | fast (< 100 ms) | `scripts/verify` or `npm test` |
| Panel forms, spam, Bcc | `test/panel.test.js` | fast | `scripts/verify` or `npm test` |
| Syntax check | `npm run lint` (`node --check`) | fast | `scripts/verify` |
| Live panel and API | not automated | manual | see the runbooks |

The runner is `node --test` (Node's built-in test runner); there are no test dependencies.

## Fixtures and environments
Fixtures are inline: small HTML strings that mimic the panel's forms and JSON responses. They contain no real data.
Never commit a real cookie, API key or panel page. For a live check, use a throwaway domain and restore the original
values; `docs/web-panel.md` lists the forms.

## CI
`ci.yml` runs `bash scripts/verify` on Ubuntu, macOS and Windows with Node 22 and 24. `security.yml` adds CodeQL,
Gitleaks over the full history, dependency review on pull requests, and a "pentest" job: Semgrep (OWASP and Node.js
rule sets), a grep that fails if any source uses a shell, and a check that a fake API key never appears in error output.
The nine required checks are `verify` ×6, `codeql`, `secrets` and `pentest`.

## Definition of a passing release
`scripts/verify` green in CI on all six platform combinations, the security workflow green, and the release workflow's
smoke test (help prints, no secret in error output) green.
