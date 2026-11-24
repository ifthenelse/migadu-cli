# Intent: migadu-cli

Durable statement of intent. It is not a requirements backlog.

## Business problem
Migadu hosts email for many small projects, but it has no official command line, and its REST API is documented as
early beta and leaves out several settings (plan, catchall, spam filtering, Bcc captures). People who set up a new
project, or run many domains, must click through the admin panel or hand-write `curl` calls. The only dedicated CLI
(jacksonrakena/migadu-cli) was archived in December 2021.

Who has it: developers and sysadmins who manage Migadu mailboxes for their own apps (homelabbers, startup founders)
and small agencies that operate many client domains.

## Intended outcomes
- Create and manage mailboxes, aliases, rewrites, identities, forwardings and domains from a terminal or a script.
- Reach the settings the REST API does not expose, through an opt-in admin-panel session.
- Keep the API key out of dotfiles by reading it from a password manager at run time.
- Run the same way on Windows, macOS and Linux as one native binary with no runtime to install.

## Non-goals
- Not an official Migadu product and not affiliated with Migadu.
- Not a mail client: no sending, reading or syncing mail.
- Not a hosted service, and no telemetry.
- No storing of passwords or API keys by the tool itself.
- No scraping of the panel beyond the documented, opt-in web mode.

## Success measures
| Measure | Target | Source |
| --- | --- | --- |
| GitHub stars | 5 within 150 days of the first release | GitHub repository |
| Release downloads | 50 within 150 days of the first release | GitHub release assets |
| Outside issues or pull requests | at least 1 | GitHub |

The targets come from the owner (GTM plan, 2026-11-24). They are deliberately small for a niche tool.

## Accountable owners
- Andrea Collet: maintainer and product owner.
