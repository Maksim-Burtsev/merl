use super::{Task, at, edited_since_unpacked, selected_on_one_line};
use crate::app::{App, Focus, Mode, PickerKind};

const INIT_LINE: usize = 12;
const CALL_LINE: usize = 13;
const CLASS_LINE: usize = 9;
const IMPORT_LINE: usize = 5;
const REMOVE_LINE: usize = 39;
const REMOVE_END: usize = 46;
const PARA_UP_LINE: usize = 38;
const PARA_DOWN_LINE: usize = 47;
const LOAD_CONFIG_DEF: usize = 18;
const TODO_LINE: usize = 30;
const CLI_IMPORT_LINE: usize = 6;
const NOTES_END: usize = 297;

const TEST_FILE: &str = "tests/test_store.py";

fn half_screen(a: &App) -> usize {
    (a.view_h / 2).max(1)
}

fn lines_selected(a: &App, from1: usize, to1: usize) -> bool {
    a.selection()
        .is_some_and(|(f, t)| (f.0.key() + 1, t.0.key() + 1) == (from1, to1))
}

fn picker_row(a: &App, kind: PickerKind) -> Option<usize> {
    let p = a.picker.as_ref().filter(|_| a.mode == Mode::Picker(kind))?;
    Some(p.selected)
}

fn tree_on(a: &App, path: &str) -> bool {
    a.focus == Focus::Tree
        && a.show_tree
        && a.tree
            .selected()
            .is_some_and(|n| n.path.as_os_str() == path)
}

fn tree_rows(a: &App, from: &str) -> Option<(usize, usize)> {
    let vis = a.tree.visible();
    let row = |p: &dyn Fn(usize) -> bool| vis.iter().position(|&i| p(i));
    let at = row(&|i| i == a.tree.cursor)?;
    let from = row(&|i| a.tree.nodes[i].path.as_os_str() == from)?;
    (a.focus == Focus::Tree && a.show_tree).then_some((at, from))
}

fn folder_open(a: &App, path: &str) -> bool {
    a.tree
        .nodes
        .iter()
        .any(|n| n.path.as_os_str() == path && n.expanded)
}

fn word_deleted(a: &App) -> bool {
    a.mode == Mode::Edit && a.buf.lines[CALL_LINE - 1].trim() == "self.config = config or ()"
}

pub const POOL: &[Task] = &[
    Task {
        key: "o",
        title: "Open a file",
        tutor: "Press `o` (or Ctrl+E), type `store`, and press Enter to open store.py.",
        drill: "Open store.py.",
        start: None,
        keys: "",
        answer: "ostore<Enter>",
        done: |a| at(a, "store.py"),
    },
    Task {
        key: "Ctrl+N",
        title: "New file",
        tutor: "Ctrl+N makes a new file: type its path from the project root, `todo.txt`, and \
                Enter creates it and starts editing it.",
        drill: "Make a new file, `todo.txt`, in the project root.",
        start: None,
        keys: "",
        answer: "<C-n>todo.txt<Enter>",
        done: |a| at(a, "todo.txt"),
    },
    Task {
        key: "/",
        title: "Find in the file",
        tutor: "Press `/` (or Ctrl+F), type `load_config`, Enter. The search runs as you type.",
        drill: "Find `load_config` in this file.",
        start: Some(("store.py", 1, "")),
        keys: "",
        answer: "/load_config<Enter>",
        done: |a| {
            a.mode == Mode::Normal
                && a.find_re
                    .as_ref()
                    .is_some_and(|r| r.as_str() == "load_config")
        },
    },
    Task {
        key: "n",
        title: "Next match",
        tutor: "The file is searched for `load_config`. `n` goes to the next match, `N` to the \
                previous one; `/` again brings the query back, selected, to refine it. Press \
                `n`: the call inside NoteStore.",
        drill: "Go on to the next match of `load_config`: the call inside NoteStore.",
        start: Some(("store.py", 1, "")),
        keys: "/load_config<Enter>",
        answer: "n",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE,
    },
    Task {
        key: "N",
        title: "Previous match",
        tutor: "The file is searched for `load_config`, and `N` goes back to the previous match. \
                Press it: the import at the top.",
        drill: "Go back to the match of `load_config` before this one: the import at the top.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "/load_config<Enter>",
        answer: "N",
        done: |a| at(a, "store.py") && a.line + 1 == IMPORT_LINE && a.find_re.is_some(),
    },
    Task {
        key: "*",
        title: "Find the word under the cursor",
        tutor: "`*` finds the word under the cursor as a whole word, in the same case, and goes \
                to the next one; `n` and `N` walk the rest. Press it on `config`: it skips \
                `Config` and stops on `self.config` below.",
        drill: "Find `config` as a whole word, in the same case, and go to its next use: \
                `self.config` below.",
        start: Some(("store.py", INIT_LINE, "config")),
        keys: "",
        answer: "*",
        done: |a| {
            at(a, "store.py")
                && a.line + 1 == CALL_LINE
                && a.find_re
                    .as_ref()
                    .is_some_and(|r| r.as_str() == r"(?-u:\b)config(?-u:\b)")
        },
    },
    Task {
        key: "s",
        title: "Search the project",
        tutor: "`s` searches every file as you type. Type `TODO`, then Enter on the result.",
        drill: "Go to the TODO, wherever it is in the project.",
        start: Some(("cli.py", CLI_IMPORT_LINE, "load_config")),
        keys: "",
        answer: "sTODO<Enter>",
        done: |a| at(a, "models.py") && a.line + 1 == TODO_LINE,
    },
    Task {
        key: "d",
        title: "Go to definition",
        tutor: "With the cursor on `load_config`, press `d` to jump to its definition. \
                The status line says how it was found: via the import at the top, in config.py. \
                `d` reads Makefiles, Terraform, Dockerfiles and YAML too, and on a declaration \
                it lists what implements it.",
        drill: "Jump to where `load_config` is defined.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "d",
        done: |a| at(a, "config.py") && a.line + 1 == LOAD_CONFIG_DEF,
    },
    Task {
        key: "D",
        title: "Project symbols",
        tutor: "`D` lists every class and function of the project. Type `NoteStore` and press \
                Enter.",
        drill: "Go to the class NoteStore.",
        start: Some(("models.py", TODO_LINE, "TODO")),
        keys: "",
        answer: "DNoteStore<Enter>",
        done: |a| at(a, "store.py") && a.line + 1 == CLASS_LINE,
    },
    Task {
        key: "u",
        title: "Usages",
        tutor: "`u` lists every use of the word under the cursor, the declaration \
                first and the tests last. Press `u`, then pick the hit in cli.py with Down and \
                Enter.",
        drill: "Go to where cli.py uses `load_config`.",
        start: Some(("config.py", LOAD_CONFIG_DEF, "load_config")),
        keys: "",
        answer: "u<Down><Enter>",
        done: |a| at(a, "cli.py"),
    },
    Task {
        key: "[",
        title: "Back",
        tutor: "`d` jumped from store.py to config.py. `[` goes back in the jump history, to where \
                you came from.",
        drill: "Go back to the call in store.py this jump came from.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "d",
        answer: "[",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE,
    },
    Task {
        key: "]",
        title: "Forward",
        tutor: "`d` jumped to config.py and `[` came back. `]` goes forward again, back into \
                config.py.",
        drill: "Return to the definition in config.py you were just at.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "d[",
        answer: "]",
        done: |a| at(a, "config.py"),
    },
    Task {
        key: ":",
        title: "Go to line",
        tutor: "`:` (or Ctrl+G), then a number: type `42` and press Enter.",
        drill: "Go to line 42.",
        start: Some(("store.py", CLASS_LINE, "")),
        keys: "",
        answer: ":42<Enter>",
        done: |a| at(a, "store.py") && a.line + 1 == 42,
    },
    Task {
        key: "t",
        title: "File tree",
        tutor: "`t` shows and hides the file tree.",
        drill: "Show the file tree.",
        start: Some(("store.py", REMOVE_LINE, "")),
        keys: "",
        answer: "t",
        done: |a| a.show_tree,
    },
    Task {
        key: "w",
        title: "Long lines",
        tutor: "Long lines wrap at the edge of the pane, which takes a table apart, so a .csv file \
                is cut there instead: `\u{203a}` marks a cut line and the view follows the cursor \
                sideways. `w` flips that for the open file until merl quits, and the status bar \
                says `nowrap` while its lines are cut. Press `w` to wrap this table.",
        drill: "Let the long lines of this table wrap.",
        start: Some(("export.csv", 1, "")),
        keys: "",
        answer: "w",
        done: |a| at(a, "export.csv") && !a.nowrap(),
    },
    Task {
        key: "p",
        title: "Markdown, rendered",
        tutor: "Agents write their plans in Markdown, like PLAN.md. `p` shows it rendered: headings, \
                lists, the table drawn in lines, the code in colour. `p` again shows the source, at \
                the same place, and `Enter` edits there. Press `p`.",
        drill: "Read this plan rendered instead of as its source.",
        start: Some(("PLAN.md", 1, "")),
        keys: "",
        answer: "p",
        done: |a| at(a, "PLAN.md") && a.previewing(),
    },
    Task {
        key: "f",
        title: "Fold a function",
        tutor: "In a long file, a function you have read takes the room of the next one. `f` folds \
                the function the cursor is in into its first line, ending in `\u{22ef}`, and the \
                arrows step over it; `f` on that line unfolds it, and so does a jump into it. \
                Press `f` to fold `remove`.",
        drill: "Hide the body of `remove` under its first line.",
        start: Some(("store.py", REMOVE_LINE + 2, "")),
        keys: "",
        answer: "f",
        done: |a| at(a, "store.py") && a.collapsed_at(REMOVE_LINE - 1).is_some(),
    },
    Task {
        key: "Tab",
        title: "Focus the tree",
        tutor: "Tab switches the focus between the code and the tree. Press it once.",
        drill: "Move the focus into the file tree.",
        start: Some(("store.py", REMOVE_LINE, "")),
        keys: "t",
        answer: "<Tab>",
        done: |a| a.focus == Focus::Tree,
    },
    Task {
        key: "Enter",
        title: "Edit",
        tutor: "Enter starts editing at the cursor and the cursor becomes a bar: type `#hi` and \
                press Esc, the block is back. merl saves a second after you stop typing; Ctrl+S \
                saves now, Ctrl+R reloads the file from disk.",
        drill: "Type `#hi` at the top of this file, then go back to moving around it.",
        start: Some((TEST_FILE, 1, "")),
        keys: "",
        answer: "<Enter>#hi<Esc>",
        done: |a| a.mode == Mode::Normal && edited_since_unpacked(a),
    },
    Task {
        key: "Ctrl+Z",
        title: "Undo",
        tutor: "`#hi` was typed at the top. Ctrl+Z takes an edit back, Ctrl+Y brings it again. \
                Press Ctrl+Z once: the file is as it was, and a second later so is the disk.",
        drill: "Take back the `#hi` typed at the top.",
        start: Some((TEST_FILE, 1, "")),
        keys: "<Enter>#hi<Esc>",
        answer: "<C-z>",
        done: |a| at(a, TEST_FILE) && a.mode == Mode::Normal && !edited_since_unpacked(a),
    },
    Task {
        key: "Ctrl+Y",
        title: "Redo",
        tutor: "`#hi` was typed at the top and taken back. Ctrl+Y brings back what Ctrl+Z took: \
                press it.",
        drill: "Bring back the `#hi` that was taken back.",
        start: Some((TEST_FILE, 1, "")),
        keys: "<Enter>#hi<Esc><C-z>",
        answer: "<C-y>",
        done: |a| at(a, TEST_FILE) && a.mode == Mode::Normal && edited_since_unpacked(a),
    },
    Task {
        key: "Ctrl+C",
        title: "Copy",
        tutor: "Ctrl+C copies the selection to the clipboard or, with nothing selected, the whole \
                line. Press it to copy this line.",
        drill: "Copy this whole line to the clipboard.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<C-c>",
        done: |a| {
            a.clipboard
                .as_deref()
                .is_some_and(|c| c == "        self.config = config or load_config()\n")
        },
    },
    Task {
        key: "Edit: Ctrl+X",
        title: "Cut",
        tutor: "While editing, Ctrl+X cuts the selection or, with nothing selected, the whole \
                line. Press it to cut this line.",
        drill: "You are typing. Cut this whole line to the clipboard.",
        start: Some(("store.py", CALL_LINE, "")),
        keys: "<Enter>",
        answer: "<C-x>",
        done: |a| {
            a.mode == Mode::Edit
                && a.buf.lines[CALL_LINE - 1].trim() == "self.notes: list[Note] = self._read()"
                && a.clipboard
                    .as_deref()
                    .is_some_and(|c| c.contains("load_config"))
        },
    },
    Task {
        key: "Edit: Alt+Backspace",
        title: "Delete a word",
        tutor: "You are editing, right after `load_config`. Alt+Backspace (Option on a Mac) \
                deletes the word before the cursor.",
        drill: "You are typing. Delete `load_config` before the cursor.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "<A-Right><Enter>",
        answer: "<A-BS>",
        done: word_deleted,
    },
    Task {
        key: "Edit: Alt+Delete",
        title: "Delete the next word",
        tutor: "You are editing, right before `load_config`. Alt+Delete deletes the word after the \
                cursor.",
        drill: "You are typing. Delete `load_config` after the cursor.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "<Enter>",
        answer: "<A-Del>",
        done: word_deleted,
    },
    Task {
        key: "Arrows",
        title: "Move",
        tutor: "Arrows move the cursor; Up and Down go by screen row, so a wrapped line is walked \
                a row at a time. Press Down.",
        drill: "Move the cursor to the line below.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<Down>",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE + 1,
    },
    Task {
        key: "Shift+Up",
        title: "Select up",
        tutor: "Shift+Up extends the selection by a screen row upwards. Press it to select up to \
                the line above.",
        drill: "Select from the cursor up to the same place on the line above.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<S-Up>",
        done: |a| lines_selected(a, CALL_LINE - 1, CALL_LINE),
    },
    Task {
        key: "Shift+Down",
        title: "Select down",
        tutor: "Shift+Down extends the selection by a screen row downwards. Press it to select \
                down to the line below.",
        drill: "Select from the cursor down to the same place on the line below.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<S-Down>",
        done: |a| lines_selected(a, CALL_LINE, CALL_LINE + 1),
    },
    Task {
        key: "Shift+Left",
        title: "Select a character back",
        tutor: "Shift+Left extends the selection by a character to the left. Press it to select \
                the `g` before the cursor.",
        drill: "Select the `g` just before the cursor.",
        start: Some(("store.py", CALL_LINE, "()")),
        keys: "",
        answer: "<S-Left>",
        done: |a| selected_on_one_line(a) == Some("g"),
    },
    Task {
        key: "Shift+Right",
        title: "Select a character",
        tutor: "Shift+Right extends the selection by a character to the right. Press it to select \
                the `(` under the cursor.",
        drill: "Select the `(` under the cursor.",
        start: Some(("store.py", CALL_LINE, "()")),
        keys: "",
        answer: "<S-Right>",
        done: |a| selected_on_one_line(a) == Some("("),
    },
    Task {
        key: "Alt+Left",
        title: "Word left",
        tutor: "Alt+Left (Option on a Mac) moves to the start of the word before the cursor. \
                Press it to land on `or`.",
        drill: "Move the cursor back to the start of `or`.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<A-Left>",
        done: |a| {
            at(a, "store.py")
                && a.line_str()
                    .get(a.col..)
                    .is_some_and(|s| s.starts_with("or "))
        },
    },
    Task {
        key: "Alt+Right",
        title: "Word right",
        tutor: "Alt+Right moves to the end of the word under the cursor, or of the next one. \
                Press it to land right after `load_config`.",
        drill: "Move the cursor to just after `load_config`.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<A-Right>",
        done: |a| at(a, "store.py") && a.line_str().get(a.col..) == Some("()"),
    },
    Task {
        key: "Alt+Shift+Left",
        title: "Select words back",
        tutor: "Alt+Shift+Left extends the selection by a word to the left. Press it three times \
                to select `config or load_config`.",
        drill: "Select `config or load_config`, back from the cursor.",
        start: Some(("store.py", CALL_LINE, "()")),
        keys: "",
        answer: "<A-S-Left><A-S-Left><A-S-Left>",
        done: |a| selected_on_one_line(a) == Some("config or load_config"),
    },
    Task {
        key: "Alt+Shift+Right",
        title: "Select words",
        tutor: "Alt+Shift+Right extends the selection by a word to the right. Press it three times \
                to select `config or load_config`.",
        drill: "Select `config or load_config`, on from the cursor.",
        start: Some(("store.py", CALL_LINE, "config or")),
        keys: "",
        answer: "<A-S-Right><A-S-Right><A-S-Right>",
        done: |a| selected_on_one_line(a) == Some("config or load_config"),
    },
    Task {
        key: "Ctrl+Shift+Left",
        title: "Select to the line start",
        tutor: "Ctrl+Shift+Left extends the selection to the start of the screen row, and pressed \
                again to the start of the line. Press it once.",
        drill: "Select everything on this line before the cursor.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<C-S-Left>",
        done: |a| selected_on_one_line(a) == Some("        self.config = config or "),
    },
    Task {
        key: "Ctrl+Shift+Right",
        title: "Select to the line end",
        tutor: "Ctrl+Shift+Right extends the selection to the end of the screen row, and pressed \
                again to the end of the line. Press it once.",
        drill: "Select the rest of this line from the cursor on.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<C-S-Right>",
        done: |a| selected_on_one_line(a) == Some("load_config()"),
    },
    Task {
        key: "Shift+PgUp",
        title: "Select a screen up",
        tutor: "Shift+PgUp extends the selection a whole screen up. Press it.",
        drill: "Select a whole screen up from the cursor.",
        start: Some(("notes.json", NOTES_END, "")),
        keys: "",
        answer: "<S-PgUp>",
        done: |a| {
            at(a, "notes.json")
                && a.selection().is_some_and(|(f, t)| {
                    f.0.key() + 1 + a.view_h <= NOTES_END && t.0.key() + 1 == NOTES_END
                })
        },
    },
    Task {
        key: "Shift+PgDn",
        title: "Select a screen down",
        tutor: "Shift+PgDn extends the selection a whole screen down. Press it.",
        drill: "Select a whole screen down from the cursor.",
        start: Some(("notes.json", 1, "")),
        keys: "",
        answer: "<S-PgDn>",
        done: |a| {
            at(a, "notes.json")
                && a.selection()
                    .is_some_and(|(f, t)| f.0.key() == 0 && t.0.key() >= a.view_h)
        },
    },
    Task {
        key: "Ctrl+Shift+Home",
        title: "Select to the file start",
        tutor: "Ctrl+Shift+Home extends the selection to the start of the file. Press it.",
        drill: "Select everything from the start of the file to the cursor.",
        start: Some(("store.py", 42, "")),
        keys: "",
        answer: "<C-S-Home>",
        done: |a| at(a, "store.py") && lines_selected(a, 1, 42) && (a.line, a.col) == (0, 0),
    },
    Task {
        key: "Ctrl+Shift+End",
        title: "Select to the file end",
        tutor: "Ctrl+Shift+End extends the selection to the end of the file. Press it.",
        drill: "Select everything from the cursor to the end of the file.",
        start: Some(("store.py", CLASS_LINE, "")),
        keys: "",
        answer: "<C-S-End>",
        done: |a| {
            at(a, "store.py")
                && lines_selected(a, CLASS_LINE, a.buf.lines.len())
                && a.col == a.line_str().len()
        },
    },
    Task {
        key: "v",
        title: "Select",
        tutor: "`v` selects the word under the cursor, and each press grows it: inside the \
                brackets or quotes, with them, the line, the block, the function. Press `v` three \
                times to select `remove`.",
        drill: "Select the whole function `remove`.",
        start: Some(("store.py", REMOVE_LINE, "remove")),
        keys: "",
        answer: "vvv",
        done: |a| lines_selected(a, REMOVE_LINE, REMOVE_END),
    },
    Task {
        key: "V",
        title: "Select less",
        tutor: "`v` grew the selection to the whole function `remove`. `V` steps it back to what \
                the press before had: the line. Press it.",
        drill: "Take the selection back to the line it held before.",
        start: Some(("store.py", REMOVE_LINE, "remove")),
        keys: "vvv",
        answer: "V",
        done: |a| lines_selected(a, REMOVE_LINE, REMOVE_LINE),
    },
    Task {
        key: "Ctrl+D",
        title: "Half a screen down",
        tutor: "Ctrl+D moves the cursor and the view half a screen down, so half of what you read \
                stays in sight. Press it.",
        drill: "Move half a screen down.",
        start: Some(("notes.json", 1, "")),
        keys: "",
        answer: "<C-d>",
        done: |a| at(a, "notes.json") && a.line >= half_screen(a),
    },
    Task {
        key: "Ctrl+U",
        title: "Half a screen up",
        tutor: "Ctrl+U moves the cursor and the view half a screen up. Press it.",
        drill: "Move half a screen up.",
        start: Some(("notes.json", NOTES_END, "")),
        keys: "",
        answer: "<C-u>",
        done: |a| at(a, "notes.json") && a.line + 1 + half_screen(a) <= NOTES_END,
    },
    Task {
        key: "{",
        title: "Previous paragraph",
        tutor: "`{` jumps to the previous blank line: in code, the one above the function. \
                Press it once to reach the line above `remove`.",
        drill: "Go up to the blank line above `remove`.",
        start: Some(("store.py", PARA_DOWN_LINE, "")),
        keys: "",
        answer: "{",
        done: |a| at(a, "store.py") && a.line + 1 == PARA_UP_LINE,
    },
    Task {
        key: "}",
        title: "Next paragraph",
        tutor: "`}` jumps to the next blank line. In code that is the end of the current \
                function: press it once.",
        drill: "Go down to the blank line that ends this function.",
        start: Some(("store.py", 42, "")),
        keys: "",
        answer: "}",
        done: |a| at(a, "store.py") && a.line + 1 == PARA_DOWN_LINE,
    },
    Task {
        key: "PgUp",
        title: "A screen up",
        tutor: "PgUp moves the cursor a whole screen up. Press it.",
        drill: "Move a whole screen up.",
        start: Some(("notes.json", NOTES_END, "")),
        keys: "",
        answer: "<PgUp>",
        done: |a| at(a, "notes.json") && a.line + 1 + a.view_h <= NOTES_END,
    },
    Task {
        key: "PgDn",
        title: "A screen down",
        tutor: "PgDn moves the cursor a whole screen down. Press it.",
        drill: "Move a whole screen down.",
        start: Some(("notes.json", 1, "")),
        keys: "",
        answer: "<PgDn>",
        done: |a| at(a, "notes.json") && a.line >= a.view_h,
    },
    Task {
        key: "Home",
        title: "Line start",
        tutor: "Home goes to the start of the screen row and, pressed there, of the line. Press \
                it.",
        drill: "Go to the start of this line.",
        start: Some(("store.py", CALL_LINE, "load_config")),
        keys: "",
        answer: "<Home>",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE && a.col == 0,
    },
    Task {
        key: "End",
        title: "Line end",
        tutor: "End goes to the end of the screen row and, pressed there, of the line. Press it.",
        drill: "Go to the end of this line.",
        start: Some(("store.py", CALL_LINE, "self")),
        keys: "",
        answer: "<End>",
        done: |a| at(a, "store.py") && a.line + 1 == CALL_LINE && a.col == a.line_str().len(),
    },
    Task {
        key: "Ctrl+Home",
        title: "File start",
        tutor: "Ctrl+Home goes to the start of the file. Press it.",
        drill: "Go to the very start of the file.",
        start: Some(("store.py", 42, "")),
        keys: "",
        answer: "<C-Home>",
        done: |a| at(a, "store.py") && (a.line, a.col) == (0, 0),
    },
    Task {
        key: "Ctrl+End",
        title: "File end",
        tutor: "Ctrl+End goes to the end of the file. Press it.",
        drill: "Go to the very end of the file.",
        start: Some(("store.py", CLASS_LINE, "")),
        keys: "",
        answer: "<C-End>",
        done: |a| {
            at(a, "store.py") && a.line + 1 == a.buf.lines.len() && a.col == a.line_str().len()
        },
    },
    Task {
        key: "Esc",
        title: "Close it",
        tutor: "`?` lists every key merl knows, and it is open now. Esc closes any overlay, and \
                clears the selection and the find pattern. Press Esc.",
        drill: "Close the list of keys.",
        start: Some(("export.csv", 1, "")),
        keys: "?",
        answer: "<Esc>",
        done: |a| a.mode == Mode::Normal,
    },
    Task {
        key: "Tree: Up",
        title: "Up the tree",
        tutor: "In the tree, Up moves the cursor a row up. Press it to reach the `tests` folder.",
        drill: "Move the tree's cursor up onto the `tests` folder.",
        start: Some(("cli.py", 1, "")),
        keys: "t<Tab>",
        answer: "<Up>",
        done: |a| tree_on(a, "tests"),
    },
    Task {
        key: "Tree: Down",
        title: "Down the tree",
        tutor: "In the tree, Down moves the cursor a row down. Press it to reach config.py.",
        drill: "Move the tree's cursor down onto config.py.",
        start: Some(("cli.py", 1, "")),
        keys: "t<Tab>",
        answer: "<Down>",
        done: |a| tree_on(a, "config.py"),
    },
    Task {
        key: "Tree: Enter",
        title: "Open from the tree",
        tutor: "Up and Down move in the tree, Right unfolds a folder and Left folds it; Enter \
                opens the file under the cursor. It is on test_store.py: press Enter.",
        drill: "Open the file under the tree's cursor.",
        start: Some(("store.py", REMOVE_LINE, "")),
        keys: "t<Tab><Up><Up><Up><Up><Up><Up><Up><Up><Right><Down>",
        answer: "<Enter>",
        done: |a| at(a, TEST_FILE),
    },
    Task {
        key: "Tree: Left",
        title: "Fold a folder",
        tutor: "In the tree, Left folds the folder under the cursor, or goes up to the folder a \
                file is in. Press it to fold `tests`.",
        drill: "Fold the `tests` folder in the tree.",
        start: Some((TEST_FILE, 1, "")),
        keys: "t<Tab><Up>",
        answer: "<Left>",
        done: |a| tree_on(a, "tests") && !folder_open(a, "tests"),
    },
    Task {
        key: "Tree: Right",
        title: "Unfold a folder",
        tutor: "In the tree, Right unfolds the folder under the cursor. Press it to unfold \
                `tests`.",
        drill: "Unfold the `tests` folder in the tree.",
        start: Some(("cli.py", 1, "")),
        keys: "t<Tab><Up>",
        answer: "<Right>",
        done: |a| tree_on(a, "tests") && folder_open(a, "tests"),
    },
    Task {
        key: "Tree: PgDn",
        title: "A page down the tree",
        tutor: "In the tree, PgDn moves the cursor a page down, as it does in the code. Press it.",
        drill: "Move the tree's cursor a whole page down.",
        start: Some(("cli.py", 1, "")),
        keys: "t<Tab>",
        answer: "<PgDn>",
        done: |a| tree_rows(a, "cli.py").is_some_and(|(at, from)| at >= from + 2),
    },
    Task {
        key: "Tree: PgUp",
        title: "A page up the tree",
        tutor: "In the tree, PgUp moves the cursor a page up, as it does in the code. Press it.",
        drill: "Move the tree's cursor a whole page up.",
        start: Some(("store.py", 1, "")),
        keys: "t<Tab>",
        answer: "<PgUp>",
        done: |a| tree_rows(a, "store.py").is_some_and(|(at, from)| at + 2 <= from),
    },
    Task {
        key: "Tree: Ctrl+D",
        title: "Half a page down the tree",
        tutor: "In the tree, Ctrl+D moves the cursor half a page down, as it does in the code. \
                Press it.",
        drill: "Move the tree's cursor half a page down.",
        start: Some(("cli.py", 1, "")),
        keys: "t<Tab>",
        answer: "<C-d>",
        done: |a| tree_rows(a, "cli.py").is_some_and(|(at, from)| at >= from + 2),
    },
    Task {
        key: "Tree: Ctrl+U",
        title: "Half a page up the tree",
        tutor: "In the tree, Ctrl+U moves the cursor half a page up, as it does in the code. \
                Press it.",
        drill: "Move the tree's cursor half a page up.",
        start: Some(("store.py", 1, "")),
        keys: "t<Tab>",
        answer: "<C-u>",
        done: |a| tree_rows(a, "store.py").is_some_and(|(at, from)| at + 2 <= from),
    },
    Task {
        key: "Tree: Home",
        title: "The tree's first row",
        tutor: "In the tree, Home goes to the first row, with Ctrl or without. Press it to reach \
                the `tests` folder.",
        drill: "Move the tree's cursor to its first row.",
        start: Some(("store.py", 1, "")),
        keys: "t<Tab>",
        answer: "<Home>",
        done: |a| tree_on(a, "tests"),
    },
    Task {
        key: "Tree: End",
        title: "The tree's last row",
        tutor: "In the tree, End goes to the last row, with Ctrl or without. Press it to reach \
                store.py.",
        drill: "Move the tree's cursor to its last row.",
        start: Some(("cli.py", 1, "")),
        keys: "t<Tab>",
        answer: "<End>",
        done: |a| tree_on(a, "store.py"),
    },
    Task {
        key: "Tree: {",
        title: "Up to a folder",
        tutor: "In the tree, `{` goes up to the previous folder, as it goes up a paragraph in the \
                code. Press it to reach the `tests` folder.",
        drill: "Move the tree's cursor up to the folder above it, in one key.",
        start: Some(("store.py", 1, "")),
        keys: "t<Tab>",
        answer: "{",
        done: |a| tree_on(a, "tests"),
    },
    Task {
        key: "Tree: }",
        title: "Down to a folder",
        tutor: "In the tree, `}` goes down to the next folder, and past the last one to the last \
                row. No folder is below `tests`: press `}` to reach store.py.",
        drill: "Move the tree's cursor down past the last folder, in one key.",
        start: Some((TEST_FILE, 1, "")),
        keys: "t<Tab><Up>",
        answer: "}",
        done: |a| tree_on(a, "store.py"),
    },
    Task {
        key: "Picker: Up",
        title: "Up a list",
        tutor: "In a list, Up moves the selection a row up. These are the usages of \
                `load_config`: press Up to go back to its declaration.",
        drill: "Move the selection back up to the declaration.",
        start: Some(("config.py", LOAD_CONFIG_DEF, "load_config")),
        keys: "u<Down>",
        answer: "<Up>",
        done: |a| picker_row(a, PickerKind::Usages) == Some(0),
    },
    Task {
        key: "Picker: Down",
        title: "Down a list",
        tutor: "In a list, Down moves the selection a row down. These are the usages of \
                `load_config`: press Down to reach the import in cli.py.",
        drill: "Select the import of `load_config` in cli.py.",
        start: Some(("config.py", LOAD_CONFIG_DEF, "load_config")),
        keys: "u",
        answer: "<Down>",
        done: |a| {
            picker_row(a, PickerKind::Usages).is_some()
                && a.picker
                    .as_ref()
                    .and_then(|p| p.current())
                    .is_some_and(|it| it.path.as_os_str() == "cli.py" && it.line == CLI_IMPORT_LINE)
        },
    },
    Task {
        key: "Picker: Enter",
        title: "Accept",
        tutor: "In a list, Enter opens the row that is selected: here the import of `load_config` \
                in cli.py. Press Enter.",
        drill: "Open the use of `load_config` that is selected.",
        start: Some(("config.py", LOAD_CONFIG_DEF, "load_config")),
        keys: "u<Down>",
        answer: "<Enter>",
        done: |a| at(a, "cli.py") && a.line + 1 == CLI_IMPORT_LINE,
    },
    Task {
        key: "Picker: Esc",
        title: "Keep or put back",
        tutor: "`T` lists the themes and repaints merl in the one under the cursor as it moves: \
                one is on show now. Enter keeps it and saves it to the config file; Esc closes a \
                list without taking anything, and here puts the old theme back. Press Esc.",
        drill: "Put the old theme back.",
        start: Some(("export.csv", 1, "")),
        keys: "T<Down>",
        answer: "<Esc>",
        done: |a| a.picker.is_none() && a.mode == Mode::Normal,
    },
    Task {
        key: "Picker: PgUp",
        title: "A page up a list",
        tutor: "In a list, PgUp moves the selection a page up. These are the symbols of the \
                project: press PgUp to get back to the top.",
        drill: "Move the selection a whole page up this list.",
        start: Some(("store.py", 1, "")),
        keys: "D<PgDn>",
        answer: "<PgUp>",
        done: |a| picker_row(a, PickerKind::Symbols) == Some(0),
    },
    Task {
        key: "Picker: PgDn",
        title: "A page down a list",
        tutor: "In a list, PgDn moves the selection a page down. These are the symbols of the \
                project: press PgDn.",
        drill: "Move the selection a whole page down this list.",
        start: Some(("store.py", 1, "")),
        keys: "D",
        answer: "<PgDn>",
        done: |a| {
            a.picker.as_ref().is_some_and(|p| {
                let last = (p.counts().0 as usize).saturating_sub(1);
                let a_page_or_the_last_row = p.page().min(last);
                picker_row(a, PickerKind::Symbols)
                    .is_some_and(|row| row > 0 && row >= a_page_or_the_last_row)
            })
        },
    },
    Task {
        key: "Help: Up",
        title: "Scroll the help up",
        tutor: "`?` lists every key merl knows, and Up scrolls the list a row up. Press it.",
        drill: "Scroll the list of keys back up.",
        start: Some(("store.py", 1, "")),
        keys: "?<Down>",
        answer: "<Up>",
        done: |a| a.mode == Mode::Help && a.help_top == 0,
    },
    Task {
        key: "Help: Down",
        title: "Scroll the help down",
        tutor: "`?` lists every key merl knows, and Down scrolls the list a row down. Press it.",
        drill: "Scroll the list of keys down.",
        start: Some(("store.py", 1, "")),
        keys: "?",
        answer: "<Down>",
        done: |a| a.mode == Mode::Help && a.help_top > 0,
    },
    Task {
        key: "c",
        title: "Next change",
        tutor: "A review: an agent built PLAN.md and left its work uncommitted, the files on the \
                left. `c` goes to the next change, and past the last one into the next file, \
                ticking the one you walked off. Press `c` until you are in models.py.",
        drill: "Walk the changes of cli.py on into models.py.",
        start: Some(("cli.py", 1, "")),
        keys: "",
        answer: "cccccc",
        done: |a| at(a, "models.py"),
    },
    Task {
        key: "C",
        title: "Previous change",
        tutor: "`C` walks back: to the change before, and from the first one into the file \
                before. Press `C`: from the top of models.py back into cli.py.",
        drill: "Go back to the change before this one, in cli.py.",
        start: Some(("models.py", 1, "")),
        keys: "",
        answer: "C",
        done: |a| at(a, "cli.py"),
    },
    Task {
        key: "m",
        title: "Mark viewed",
        tutor: "`m` marks the file you are in as viewed, a tick beside it on the left; `m` again \
                takes it off. Press `m` on store.py.",
        drill: "Mark store.py as viewed.",
        start: Some(("store.py", 1, "")),
        keys: "",
        answer: "m",
        done: |a| a.viewed.contains_key(std::path::Path::new("store.py")),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tutor::SAMPLE_FILES;

    fn line(file: &str, n: usize) -> &'static str {
        let (_, text) = SAMPLE_FILES.iter().find(|(f, _)| *f == file).unwrap();
        text.lines().nth(n - 1).unwrap_or_default()
    }

    #[test]
    fn the_sample_lines_the_tasks_name_hold_what_they_are_named_after() {
        for (file, n, want) in [
            (
                "store.py",
                CALL_LINE,
                "self.config = config or load_config()",
            ),
            ("store.py", CLASS_LINE, "class NoteStore:"),
            (
                "store.py",
                IMPORT_LINE,
                "from config import Config, load_config",
            ),
            ("store.py", REMOVE_LINE, "def remove("),
            ("store.py", REMOVE_END, "return True"),
            ("config.py", LOAD_CONFIG_DEF, "def load_config("),
            ("models.py", TODO_LINE, "# TODO:"),
            ("cli.py", CLI_IMPORT_LINE, "from config import load_config"),
        ] {
            assert!(
                line(file, n).contains(want),
                "{file}:{n} is {:?}",
                line(file, n)
            );
        }
        for (n, want) in [(PARA_UP_LINE, ""), (PARA_DOWN_LINE, "")] {
            assert_eq!(line("store.py", n).trim(), want, "store.py:{n}");
        }
        let (_, notes) = SAMPLE_FILES
            .iter()
            .find(|(f, _)| *f == "notes.json")
            .unwrap();
        assert_eq!(notes.lines().count(), NOTES_END);
    }
}
