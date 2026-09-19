# Regenerate with: assets/tapes/setup.sh && assets/tapes/record.sh assets/day/3-into.tape
# (from the repository root, after cargo build --release)
Output assets/day/3-into.gif

Set Shell zsh
Set FontSize 18
Set Width 1200
Set Height 540
Set Padding 12
Set TypingSpeed 65ms
Set Framerate 25
Set Theme { "background": "#222436", "foreground": "#c8d3f5", "cursor": "#c8d3f5" }

Hide
Type "export AGENT=$PWD/assets/tapes/agent.sh PATH=$PWD/target/release:$PATH HOME=/tmp/merl-demo/home PROMPT='$ ' && cd /tmp/merl-demo/gitea && git checkout -q -- . && git clean -fdq && clear"
Enter
Sleep 1s
Type "merl --review"
Enter
Sleep 2s
Type "cc"
Sleep 500ms
Type "/"
Type "CountAssignees"
Enter
Sleep 500ms
Escape
Sleep 300ms
Show
Sleep 1.2s
Type "d"
Sleep 2.2s
Type "u"
Sleep 3s
