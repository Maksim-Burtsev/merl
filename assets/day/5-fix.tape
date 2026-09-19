# Regenerate with: assets/tapes/setup.sh && assets/tapes/record.sh assets/day/5-fix.tape
# (from the repository root, after cargo build --release)
Output assets/day/5-fix.gif

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
Type "ccc"
Sleep 500ms
Type "/"
Type "to many"
Enter
Sleep 500ms
Escape
Sleep 300ms
Show
Sleep 1.2s
Enter
Sleep 600ms
Right 2
Sleep 500ms
Type "o"
Sleep 1s
Escape
Sleep 2.2s
