export const KEYS = [
  [["o"], "Open a file (fuzzy)"],
  [["s"], "Search the project"],
  [["/"], "Find in the open file"],
  [["d"], "Go to definition of the word under the cursor, or its implementations"],
  [["u"], "Usages of the word under the cursor"],
  [["D"], "Project symbols (fuzzy)"],
  [["[", "]"], "Back / forward in the jump history"],
  [["c", "C"], "Review: next / previous hunk, on to the next file"],
  [["Alt+←", "Alt+→"], "Move one word"],
  [["Enter"], "Edit at the cursor (Esc returns to navigation)"],
  [["t"], "Show or hide the file tree"],
  [["?"], "Every other key"],
];
export const NOT = [
  "No plugins.",
  "No language server and no index. Nothing to install per language, nothing to wait for.",
  "No splits and no tabs. Your terminal has those.",
  "No mouse.",
  "No keymap to write. The keys are below and they stay.",
  "No autocomplete, no snippets, no refactoring. The agent types.",
];
