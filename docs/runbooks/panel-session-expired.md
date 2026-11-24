# Runbook: panel session expired

- **Owner:** the user running the command.
- **Applies to:** `tier`, `catchall`, `spam`, `bcc`.
- **Symptom:** `error: web session expired: sign in at admin.migadu.com and copy a fresh cookie`.

## Steps
1. Sign in at https://admin.migadu.com in a browser.
2. Open the browser developer tools, Network tab, reload, select a request to `admin.migadu.com` and copy the `Cookie` request header.
3. Store it in your password manager and reference it: `export MIGADU_WEB_COOKIE_REF='op://Private/Migadu panel/cookie'`. Or, for one command, `MIGADU_WEB_COOKIE='…' migadu tier`.

## Verification
`migadu tier` prints your plan, for example `"plan": "mini"`.

## Rollback
Nothing was changed. Sign out of the panel when you are done so the cookie stops working.
