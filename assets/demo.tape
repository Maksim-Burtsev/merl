# Regenerate with: assets/tapes/setup.sh && assets/tapes/record.sh assets/demo.tape
# (from the repository root, after cargo build --release)
Output assets/demo.gif

Set Shell zsh
Set FontSize 18
Set Width 1200
Set Height 680
Set Padding 12
Set TypingSpeed 65ms
Set Framerate 25
Set Theme { "background": "#222436", "foreground": "#c8d3f5", "cursor": "#c8d3f5" }

Hide
Type "export PATH=$PWD/target/release:$PATH HOME=/tmp/merl-demo/home PROMPT='$ ' && cd /tmp/merl-demo/gitea && git checkout -q -- . && git clean -fdq && clear"
Enter
Sleep 1s
Show

Type "merl"
Sleep 400ms
Enter
Sleep 1.3s

# open a file, fuzzy
Type "o"
Sleep 500ms
Type "serv issue.go"
Sleep 900ms
Enter
Sleep 1s

# find in the file
Type "/"
Sleep 300ms
Type "LoadPullRequest"
Sleep 400ms
Enter
Sleep 700ms
Escape
Sleep 300ms

# go to definition: the status line says how it was proven
Type "d"
Sleep 1.8s

# usages: the declaration first, the tests last
Type "u"
Sleep 2s
Escape
Sleep 400ms

# back
Type "["
Sleep 900ms

# search the project, hits as you type
Type "s"
Sleep 400ms
Type@140ms "MaxAssign"
Sleep 1.3s
Escape
Sleep 600ms
Type "q"
Sleep 800ms
Hide
Type "clear"
Enter
Sleep 500ms
Show
Sleep 300ms

# the agent's branch, as a diff over the real files
Type "merl --review"
Sleep 400ms
Enter
Sleep 1.5s
Type "c"
Sleep 1s
Type "c"
Sleep 1.2s

# from a hunk into the code it calls, and back
Type "/"
Sleep 300ms
Type "CountAssignees"
Enter
Sleep 600ms
Escape
Sleep 300ms
Type "d"
Sleep 1.8s
Type "["
Sleep 1s

# the last hunk has a typo: fix it
Type "c"
Sleep 1s
Type "/"
Sleep 300ms
Type "to many"
Enter
Sleep 600ms
Escape
Sleep 300ms
Enter
Sleep 500ms
Right 2
Sleep 400ms
Type "o"
Sleep 900ms
Escape
Sleep 1.3s
