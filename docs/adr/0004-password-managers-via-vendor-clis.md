# 0004. Password managers through their own CLIs, without a shell

- Status: accepted
- Date: 2026-11-24

## Context
Users should not keep an API key in a dotfile. The common password managers each ship a command-line client.

## Decision
Accept a reference string in `MIGADU_API_KEY_REF` or `MIGADU_WEB_COOKIE_REF` and resolve it by spawning the vendor CLI
(`op`, `bw`, `pass`, `keepassxc-cli`, `security`, `secret-tool`) with an argument array and no shell. The secret is the
first line of stdout. Standard input and error stay attached so master-password prompts work. There is no Windows
Credential Manager support; Windows users use `op:`, `bw:` or `keepassxc:`.

## Alternatives considered
- Reading each manager's vault format directly: fragile and security-sensitive.
- Storing the key in a config file with permissions: it is still a key on disk.
- A shell string such as `sh -c`: invites injection; a CI check forbids it.

## Consequences
No credential storage in the tool and no injection through the reference. The vendor CLI must be installed and
unlocked; errors say when it is missing.
