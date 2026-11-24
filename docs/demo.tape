# Demo recording script for the README GIF, for VHS (https://github.com/charmbracelet/vhs).
# Record against a throwaway domain and a test mailbox, then delete them:
#   vhs docs/demo.tape      # writes .github/assets/demo.gif
# Needs MIGADU_EMAIL set and MIGADU_API_KEY_REF pointing at your password manager.
Output .github/assets/demo.gif
Set FontSize 18
Set Width 1100
Set Height 600
Set Theme "Dracula"
Set TypingSpeed 40ms

Type "# New project, new mailboxes: done in the terminal"
Enter
Sleep 800ms
Type "export MIGADU_API_KEY_REF='op://Private/Migadu/credential'  # the key never touches a file"
Enter
Sleep 1s
Type "migadu mailboxes create -d example.org -f local_part=hello -f name=Hello -f password_method=invitation -f password_recovery_email=me@example.net"
Enter
Sleep 3s
Type "migadu aliases create -d example.org -f local_part=support -f destinations=hello@example.org"
Enter
Sleep 3s
Type "migadu mailboxes list -d example.org -o table"
Enter
Sleep 3s
Type "migadu mailboxes delete -d example.org hello --yes"
Enter
Sleep 3s
