# Runbook: panel markup changed

- **Owner:** maintainer.
- **Symptom:** a panel command fails with `form not found (page layout changed or session invalid)`, `could not find the plan…`, or `not applied`, while a fresh cookie works in the browser.

## Prerequisites
A signed-in browser, a throwaway domain, and the mapped forms in [`docs/web-panel.md`](../web-panel.md).

## Steps
1. Reproduce with a fresh cookie to rule out an expired session ([runbook](panel-session-expired.md)).
2. In the browser, open the affected page and inspect the form: its action, `_method`, `template`, field names and any controls inside HTML comments.
3. Compare with `docs/web-panel.md` and the parser in `src/web.js` and `src/forms.js`. Update the fixture in `test/panel.test.js` first so a test fails, then fix the code.
4. Run `scripts/verify`. Do a live check on the throwaway domain: read the value, change it, confirm it applied, restore it.
5. Update `docs/web-panel.md` and open a pull request.

## Verification
The new test passes, the live change applies and is restored, and CI is green.

## Rollback
Revert the pull request. If the panel is unusable for a feature, say so in the README's limitations until fixed; the REST commands are unaffected.
