# Regenerate with: vhs assets/demo.tape  (needs a release build: cargo build --release)
Output assets/demo.gif

Set Shell zsh
Set FontSize 15
Set Width 1200
Set Height 640
Set Padding 12
Set TypingSpeed 60ms
Set Theme { "background": "#222436", "foreground": "#c8d3f5", "cursor": "#c8d3f5" }

Hide
Type "./target/release/merl --theme tokyonight-moon tutor/notes"
Enter
Sleep 500ms
Show

Sleep 1.5s

# open a file, fuzzy
Type "o"
Sleep 600ms
Type "store"
Sleep 800ms
Enter
Sleep 1.2s

# find in the file
Type "/"
Sleep 400ms
Type "load_config"
Sleep 700ms
Enter
Sleep 1s

# go to definition, then back
Type "d"
Sleep 1.8s
Type "["
Sleep 1.2s

# usages of the word under the cursor
Type "u"
Sleep 1.8s
Escape
Sleep 600ms

# project symbols
Type "D"
Sleep 600ms
Type "page"
Sleep 800ms
Enter
Sleep 1.2s

# every key
Type "?"
Sleep 2s
Escape
Sleep 800ms
