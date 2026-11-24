# GTM Plan: migadu-cli
*Built with the Go2Rev methodology · 2026-11-24*

> A GTM plan is a chain of decisions: category → promise → engines → channels → commitments.

The coherence check was done inline, not by the plan-qa agent.

## 1. Executive summary
migadu-cli is a free (MIT), unofficial command line for Migadu email hosting, built and maintained by one person with 4–6 hours a week and no budget. Migadu has no official CLI, and the only dedicated one (jacksonrakena/migadu-cli) was archived in December 2021. The category bet is **"the maintained command line for Migadu"**. The promise is "New project, new mailboxes: done in the terminal, in seconds." Three engines carry it: **install-ready** (a one-line install), **search and docs** (be found by people who need it), and **community launches** (r/selfhosted, Show HN, a blog post). The first 90 days commit to six initiatives, about 55 of the roughly 65 available hours. Success is measured in adoption signals. The owner's target is 5 stars and 50 downloads in 150 days.

## 2. Business snapshot
- **Offer:** CLI for the Migadu REST API plus panel-only features (plan, catchall, spam, Bcc) via an opt-in session cookie. Credentials come from 1Password, Bitwarden, pass, KeePassXC or the OS keychain. Windows, macOS, Linux; zero npm dependencies; Node 22+ today.
- **Owner's own need:** automate mailbox creation for every new project. The owner is on Mini/monthly (verified in the admin panel) with an API key.
- **ICP:** homelabbers (primary), startup founders, agencies. Medium confidence.
- **Funnel:** repo visit, install, first command, repeat use, star or contribution. No data yet: 0 stars, v0.1.0, install is `git clone` plus Node.
- **Team and constraints:** solo maintainer, €0, panel features that can break, unofficial status. An MCP server is the intended later step.

## 3. Market map
- **Layers:** direct: the archived CLI. Category (same job, other interface): Terraform provider, MCP server, Go library. Non-competitive: the admin panel and `curl`.
- **metio/terraform-provider-migadu** (14 stars, 0BSD, pushed 2026-11-23): declarative management, very active. Weakness: Terraform state and plan/apply is heavy for one-off setup.
- **Michaelzag/migadu-mcp** (17 stars, MIT, v3.6.1, 2026-04-22): for AI assistants. Nothing for plain terminals or CI.
- **MrLemur/migadu-go** (4 stars, MIT, pushed 2026-07-26): a Go library covering all REST endpoints, no releases.
- **jacksonrakena/migadu-cli** (6 stars, archived, last push 2021-12-31): the position you inherit.
- **Positioning map:** axes are interface (library, IaC, MCP, terminal) and coverage (API only, or API plus panel features). Open space: an active terminal tool with panel features. `[GAP]` active terminal tool; `[GAP HYPOTHESIS]` credential handling and a project-bootstrap workflow.
- Sources: web search, the GitHub API, the admin panel. No Similarweb or Ahrefs.

## 4. Positioning
- **Category (confirmed):** the maintained command line for Migadu. You compete with the panel and `curl`, not with Terraform or an MCP.
- **Core promise:** "New project, new mailboxes: done in the terminal, in seconds." Fallback: "Run Migadu from the terminal, with your API key kept out of dotfiles."
- **Pillars:** (1) every Migadu function from one command, claimed once domain aliases exist; (2) your API key never sits in a file; (3) safe by default (`--yes`, panel writes re-read to confirm). Roadmap pillar, not yet a claim: one-command project setup. Cross-platform and zero-dependency is proof, not a pillar.
- **Segments:** homelabbers (secrets, repeated manual setup), startup founders (time per project), agencies (mistakes across many domains).
- **Funnel messages:** awareness: "Migadu has no official CLI. This one is maintained." Consideration: a 30-second demo and "when to use this, Terraform or the MCP." Decision: "MIT, no telemetry, zero dependencies, tested on 3 OSes."
- **Anti-messages:** never "official"; no "all of Migadu" before domain aliases; no "viral", speed or user counts without data; no stability promise for panel features; no "better than Terraform".

## 5. Demand plan
- **E1 Install-ready:** stand-alone binaries, `npx`, Homebrew, a README GIF. About 8–10 hours once. Signal within days of release.
- **E2 Search and docs:** README and docs written around real queries, a comparison page, Search Console. 1–2 hours a week. Signal in 60–90 days; volumes are `needs-benchmark`.
- **E3 Community launches:** r/selfhosted first, Show HN second, a blog post, and a Migadu-community post if the channel exists. About 4 hours each. First signal after the first launch.
- **Channels:** GitHub (high), Show HN (high, one shot), r/selfhosted (medium), blog (medium), npm and Homebrew (medium), Migadu community (low). All organic, €0.
- **Sequence:** days 1–30 install-ready; days 30–60 first launch and docs; days 60–90 Show HN and blog; later the MCP, conditional on traction.
- **Not solved:** measuring real usage (no telemetry by design), the support load, the panel-cookie risk.

## 6. Roadmap

### First 90 days
Capacity: 13 weeks × 4–6 hours = 52–78 hours. Planned: about 55.

| # | Initiative (result-phrased) | Source | Impact | Effort | Owner | Depends on | Milestone | When |
|---|---|---|---|---|---|---|---|---|
| 1 | One-line install exists for every OS | E1 | High | 10 h | Owner (Claude drafts the release workflow) | Owner approves the CI change; PR #15 merged | v0.2.0 released with downloadable binaries for 5 targets, plus `npx` or Homebrew install verified on a clean macOS, Linux and Windows machine | Weeks 1–4 |
| 2 | README first screen sells the promise | Positioning | High | 5 h | Owner reviews, Claude writes | #1 | A README with the promise, the 30-second demo GIF and a timed first command; one outside person reaches a first command in under 2 minutes | Weeks 3–4 |
| 3 | Functional and technical docs complete | E2 | Medium | 12 h (4 h owner review) | Claude writes, Owner reviews | none | `docs/` stubs (architecture, deployment, testing, security, ADRs, specifications, runbooks) filled from the code and merged | Weeks 2–7 |
| 4 | `project init` command works | Positioning | High | 6 h | Owner, with Claude | PR #15 merged | One command creates a project's mailbox, alias and forwarding, tested live on a throwaway domain | Weeks 4–6 |
| 5 | r/selfhosted launch lands | E3 | High | 4 h | Owner | #1, #2, #4 | The post draws outside engagement; the plan-level target is 5 stars and 50 downloads in 150 days | Weeks 6–8 |
| 6 | Show HN and blog post | E3 | High | 9 h | Owner | #5 survives real questions | Show HN posted with a working install and docs; blog post published | Weeks 10–12 |
| 7 | Comparison page and Search Console live | E2 | Medium | 6 h | Owner, with Claude | #3 | A "CLI vs Terraform vs MCP vs panel" page published and indexed | Weeks 8–10 |

Also in the first month: email Migadu support about panel automation and an official mention (1 hour, no initiative of its own).

### 12-month outline (directional, revisit after quarter one)
- **Months 4–6:** domain aliases, then an MCP server reusing the CLI core if launches show traction. Chase the first contributors from launch issues.
- **Months 7–12:** broader package channels (winget, AUR), a cookie-less way to read the plan if Migadu exposes one, a stable-panel-features review.

### Parked
- **Domain aliases:** the create form is mapped, but list and delete markup is unknown and a live test needs a real domain name from you. Parked until after launch, but it blocks the "every function" claim, so the claim stays out of the README until it ships.
- **Paid ads, conference talks, YouTube, newsletter:** no budget, hours or fit.
- **Social preview PNG upload:** a 10-minute task for you in repo settings, not an initiative.

## 7. Assumption and evidence ledger

### Open questions
| Type | Question | Why it matters | Confidence | Evidence | Recommendation |
|---|---|---|---|---|---|
| resolved | Success numbers | Judge the plan | high | user-declared | 5 stars and 50 downloads in 150 days |
| needs-benchmark | What a good launch looks like for a tool this small | Sets expectations for E3 | low | none sourced | Search comparable launches after the first one |
| assumption (decision risk) | ICP: homelabbers, founders, agencies | Shapes all messages | medium | user-declared, no outside signal | Test with the r/selfhosted post; see who engages |
| assumption | Panel cookie mode is acceptable to Migadu | Could break or be disallowed | medium | none | Ask Migadu support in month one |
| assumption | "No maintained CLI" is airtight | The whole category claim | medium | GitHub search only | Check Migadu's docs and forum before the launch |
| unverified-claim | "In seconds", "one command" | A claim you can't yet back | low | no timing, no `project init` | Time a demo; build #4 first |
| assumption | npm name, Homebrew tap, Migadu community channel are available | E1 and E3 depend on them | medium | unchecked | Check in week 1 |
| conflict | Building features and launching in the same 90 days | 4–6 hours a week | medium | capacity maths above | Sequenced in the roadmap; features land before the launches |

### Evidence map
| Input | Source |
|---|---|
| Goal, ICP, capacity, plan tier | user-declared; plan tier also checked in the admin panel |
| Competitor repos and activity | public source: GitHub API |
| Migadu has no official CLI, API in early beta | public source: Migadu API page, web search |
| Positioning, pillars, channel logic | agent inference |
| Launch outcomes, search volume, download rates | none (`needs-benchmark`) |

## 8. Execution readiness
- **Conversion design:** the handoff here is from visitor to first command. It needs a README that reaches a first command quickly, an issue template, and a rule for answering the first questions within a day. Skipped, launch traffic leaks at install.
- **Deployment:** a release calendar and a monthly cadence for posts and docs updates. Skipped, the launches start and stall in the first busy month.
- **Measurement:** GitHub traffic, release downloads and npm counts, reviewed weekly by the owner; no telemetry. Skipped, nobody can say which engine works.
- **Capacity note:** the plan assumes release engineering (CI, packaging), technical writing, and community moderation, all done by one person. A second contributor for packaging or docs is the first gap to expect.
