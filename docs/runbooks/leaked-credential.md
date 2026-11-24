# Runbook: leaked API key or panel cookie

- **Owner:** the account owner.
- **Symptom:** a key or cookie appeared in a log, a commit, an issue or a screenshot.

## Steps
1. **Revoke now.** API key: admin.migadu.com, My Account, API Keys, Revoke. Panel cookie: sign out of the panel (and change the password if it was shared).
2. Create a new API key and store it in your password manager; use `MIGADU_API_KEY_REF` so it is not in a file.
3. If it was committed, remove it from history only after revoking; treat the old value as public regardless. Gitleaks runs in CI on the full history.
4. Check recent activity in the admin panel for changes you did not make (mailboxes, aliases, forwardings, catchall).
5. If the leak came from this project (for example an error message that echoed a secret), report it privately and add a regression test.

## Verification
The old key returns HTTP 401 from `migadu domains list`; the new key works.

## Rollback
Not applicable: a revoked key stays revoked.
