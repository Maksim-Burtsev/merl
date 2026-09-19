# Editing

merl edits the few lines a human still types. This page is the full behaviour; the
[README](../README.md) has the short version.

Enter turns the cursor into a text cursor, Esc turns it back. In between, merl is a plain
editor with VS Code habits: letters insert, Enter splits the line and keeps its indentation, Tab
indents the way the file already does (tabs or four spaces, shown in the status bar), arrows and
Home / End move, Shift+arrows select. The letter commands are letters again once you press Esc;
the chord aliases (Ctrl+E, Ctrl+F, Ctrl+G, F12) work while editing. Find, go to line and a
cancelled prompt or picker bring you back to editing; an entry accepted in a picker ends it.

Typing over a selection replaces it, Backspace and Delete remove it. Alt+Backspace and Alt+Delete
(Option on a Mac) delete a word back and forward, here and in the prompts. Ctrl+C and Ctrl+X copy and
cut the selection (or the whole line without one) to the system clipboard through the terminal
(OSC 52: Ghostty, kitty, WezTerm, agterm, and iTerm2 once "Applications in terminal may access
clipboard" is on; Terminal.app cannot). Paste is the terminal's own Cmd+V; outside edit mode it
types into the `/`, `s` and `:` prompts and picker queries, and navigation ignores it. Ctrl+C
copies outside edit mode too and never quits: that is `q`.

The `/`, `s` and `:` prompts and picker queries are one-line editors with the same keys: arrows,
Alt+arrows by word, Home and End, Delete, Shift, Alt+Shift or Ctrl+Shift with an arrow to select, plus
the shell's Ctrl+A, Ctrl+E, Ctrl+W and Ctrl+U. Results follow every edit. While a find pattern
is active (until Esc clears it), `/` opens with it selected: type to replace it, press an arrow
or Home to edit it.

In a git repository the gutter shows what differs from the index, as VS Code's does: green for
added lines, blue for changed ones, red under a line where lines were deleted. The marks come
from `git diff` after every save and reload, so they trail an edit by the autosave delay.

Ctrl+Z and Ctrl+Y undo and redo, per file, for as long as it is open; a run of keystrokes on one
line is one step, as in VS Code. A reload from disk is one step too, as in VS Code and Vim: after
an agent writes the open file, Ctrl+Z takes back what it wrote, line endings included, and then
your own edits before it; the edits Ctrl+R drops are one Ctrl+Z away. There is no save step: edits reach the disk `autosave_delay_ms` after the last keystroke, and
at once when you leave edit mode, switch files or quit. Ctrl+S saves now. A file that changes on
disk under unsaved edits is neither reloaded nor overwritten: the status bar says so, Ctrl+S keeps
your version and Ctrl+R takes the disk's — VS Code's conflict prompt, with keys. A file deleted or
renamed on disk is changed too: the autosave never puts it back, Ctrl+S does, and Ctrl+R lets the
edits go. Until then, or
while a save keeps failing, merl stays on the file: another one does not open, and `q` has to be
pressed twice to quit without the edits. Tabs, CRLF line endings and the trailing newline come back
out as they went in; binary files, non-UTF-8 files and files with mixed line endings stay
read-only, and so does a line too long to be shown whole.
