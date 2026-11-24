# Threat model

Update when architecture or exposure changes. Scope: the CLI process on a user's machine, the credentials it handles,
and the release pipeline. Migadu's own systems are out of scope.

| Asset | Actor | Entry point | Trust boundary | Abuse case | Mitigation | Residual risk |
| --- | --- | --- | --- | --- | --- | --- |
| API key | Local attacker or log reader | Terminal output, shell history, CI logs, dotfiles | CLI ↔ user's environment | Key printed in an error or committed in a config file | Key read from env or a password manager at run time; never printed; errors name only the failure; a CI check proves a fake key does not appear in error output; Gitleaks scans history | A user who exports the key in their shell still has it in the environment |
| Session cookie (panel) | Local attacker | `MIGADU_WEB_COOKIE`, shell history | CLI ↔ environment | Cookie grants full panel access, including deleting domains | Opt-in only; `_REF` form keeps it in a password manager; the CLI never stores it; sent only to `admin.migadu.com` | Cookie is powerful and long-lived; users should use a fresh one and sign out afterwards |
| Password-manager lookup | Malicious input or compromised environment | `MIGADU_API_KEY_REF`, `MIGADU_WEB_COOKIE_REF` | CLI ↔ vendor CLI | Injection through a crafted reference; a reference echoed in an error | Vendor CLI is started with an argument array and no shell; reference scheme is validated; unknown references are rejected without echoing the value | A hostile `PATH` could substitute a vendor binary; the user controls their own `PATH` |
| Mailboxes and domains on the account | Operator mistake, scripted bug | `delete`, `catchall set`, panel writes | CLI ↔ Migadu | Accidental deletion or clearing | Deletes and clearing need `--yes`; panel writes are verified by re-reading and fail loudly; no bulk commands | A user can still pass `--yes` in a loop |
| Panel HTML | Migadu (markup change) or a network attacker | Responses from `admin.migadu.com` | CLI ↔ panel | A changed page makes the parser submit wrong fields | HTTPS only; forms are re-read after writing; unknown or missing forms abort without writing; comments are stripped so hidden controls are not submitted | The parser is regex-based and tied to current markup |
| Transport | Network attacker | HTTPS to `api.migadu.com` and `admin.migadu.com` | CLI ↔ Migadu | Interception of the key | TLS via the platform's `fetch`; 30 s timeout | Trust in the system's certificate store |
| Release artefacts | Supply-chain attacker | GitHub Actions, dependencies, binaries | Repo ↔ users | Tampered binary or workflow | Two direct Rust dependencies (`ureq` with rustls, `serde_json`), locked in `Cargo.lock`; Dependabot watches them; actions pinned to commit SHAs; Dependabot with a 7-day cooldown; branch protection with required checks; release only when the version in `Cargo.toml` changes on `main`; SHA-256 checksums published | Binaries are unsigned and macOS ones are not notarised; the crates are trusted at their locked versions |
| Maintainer account | Account takeover | GitHub | GitHub ↔ maintainer | A malicious tag or workflow change | Branch protection applies to admins; CODEOWNERS; secret scanning and push protection | Depends on the maintainer's account security (2FA) |

## Accepted risks
- Using the panel is unofficial and may be restricted by Migadu's terms; it is opt-in and documented as fragile.
- There is no telemetry, so nobody can see misuse or breakage except through issues.

## Reporting
Report a vulnerability privately through the repository's security advisories, not a public issue.
