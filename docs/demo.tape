# Record: vhs docs/demo.tape
# Edit the steps below and re-run vhs after the tool changes.

Output .github/assets/demo.gif
Require migadu

Set Shell "bash"
Set FontSize 18
Set Width 1300
Set Height 420
Set Padding 20
Set Framerate 24
Set TypingSpeed 25ms
Set Theme "Catppuccin Mocha"
Set WaitPattern /\$$/

Hide
Type "export PS1='$ '"
Enter
Type "clear"
Enter
Sleep 200ms
Show

Hide
Type 'export HOME="$(mktemp -d)"'
Enter
Wait
Type "export MIGADU_EMAIL=me@example.org MIGADU_DOMAIN=example.org"
Enter
Wait
Type "export PS1='$ '"
Enter
Wait
Type "clear"
Enter
Wait
Type "clear"
Enter
Sleep 200ms
Show

Type@25ms "migadu domains list"
Sleep 400ms
Enter
Wait
Sleep 1s

Sleep 1200ms
Type@25ms "MIGADU_API_KEY_REF=oops migadu domains list"
Sleep 400ms
Enter
Wait
Sleep 1s

Sleep 1200ms
Type@25ms "export MIGADU_API_KEY=not-a-real-key"
Sleep 400ms
Enter
Wait
Sleep 1s

Type@25ms "migadu mailboxes delete hello"
Sleep 400ms
Enter
Wait
Sleep 1s

Sleep 1200ms
Type@25ms "migadu domains list"
Sleep 400ms
Enter
Wait
Sleep 1s

Sleep 2s

Sleep 3s
