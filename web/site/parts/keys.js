import { ICONS } from "../shared/fkeys.js";

const KEYS = [
  ["o / Ctrl+E", "Open a file (fuzzy)", "Files and jumps"],
  ["Ctrl+N", "New file: type its path, Enter creates and edits it", "Files and jumps"],
  ["d / F12", "Go to definition of the word under the cursor, or its implementations", "Files and jumps"],
  ["D", "Project symbols (fuzzy)", "Files and jumps"],
  ["u / Shift+F12", "Usages of the word under the cursor", "Files and jumps"],
  ["[ / ]", "Back / forward in the jump history", "Files and jumps"],
  [": / Ctrl+G", "Go to line", "Files and jumps"],
  ["/ / Ctrl+F", "Find in the open file", "Search"],
  ["n / N", "Next / previous match", "Search"],
  ["s", "Search the project", "Search"],
  ["c / C", "Review: next / previous hunk, on to the next file", "Review"],
  ["m", "Review: mark the file as viewed, or take the mark off", "Review"],
  ["Fold: Enter", "Review: load the diff of a folded generated file", "Review"],
  ["Enter", "Edit at the cursor (Esc returns to navigation)", "Editing"],
  ["Ctrl+S", "Save now (edits are saved on their own after a pause)", "Editing"],
  ["Ctrl+R", "Reload from disk, dropping unsaved edits", "Editing"],
  ["Ctrl+Z / Ctrl+Y", "Undo / redo", "Editing"],
  ["Ctrl+C", "Copy the selection, or the line, to the clipboard", "Editing"],
  ["Edit: Ctrl+X", "Cut the selection, or the line", "Editing"],
  ["Edit: Alt+Backspace / Alt+Delete", "Delete the word before / after the cursor", "Editing"],
  ["v / V", "Grow the selection: word, quotes, brackets, line, block, file / step back", "Selection"],
  ["Shift+Up / Shift+Down", "Extend the selection by a screen row", "Selection"],
  ["Shift+Left / Shift+Right", "Extend the selection by a char", "Selection"],
  ["Alt+Shift+Left / Right", "Extend the selection by a word", "Selection"],
  ["Ctrl+Shift+Left / Right", "Extend the selection to the start / end of the screen row, then of the line", "Selection"],
  ["Shift+Home / Shift+End", "Extend the selection to the start / end of the screen row, then of the line", "Selection"],
  ["Shift+PgUp / Shift+PgDn", "Extend the selection by a screen", "Selection"],
  ["Ctrl+Shift+Home / Ctrl+Shift+End", "Extend the selection to the start / end of the file", "Selection"],
  ["Arrows", "Move the cursor; Up / Down go by screen row", "Movement"],
  ["Alt+Left / Alt+Right", "Move one word", "Movement"],
  ["Home / End", "Start / end of the screen row, then of the line", "Movement"],
  ["Ctrl+Home / Ctrl+End", "Start / end of the file", "Movement"],
  ["{ / }", "Previous / next paragraph (blank line)", "Movement"],
  ["Ctrl+D / Ctrl+U", "Move half a screen down / up", "Movement"],
  ["PgUp / PgDn", "Move one screen", "Movement"],
  ["t", "Show or hide the file tree", "Panels"],
  ["Tab", "Switch focus between tree and code", "Panels"],
  ["Tree: Up / Down", "Move", "Panels"],
  ["Tree: Enter", "Open the file, or expand the directory", "Panels"],
  ["Tree: Left / Right", "Collapse / expand", "Panels"],
  ["Picker: Up / Down", "Move", "Panels"],
  ["Picker: PgUp / PgDn", "Move one page", "Panels"],
  ["Picker: Enter", "Accept", "Panels"],
  ["Picker: Esc", "Cancel", "Panels"],
  ["?", "This help", "Panels"],
  ["Help: Up / Down", "Scroll", "Panels"],
  ["w", "Wrap long lines, or cut them at the edge and scroll sideways", "General"],
  ["p", "Show a file rendered: Markdown, CSV/TSV, JSONL, Mermaid", "General"],
  ["f", "Fold the function or block at the cursor into its first line, or unfold it", "General"],
  ["T", "Pick a theme (live preview)", "General"],
  ["Esc", "Close an overlay, leave edit mode, or clear selection and find", "General"],
  ["q", "Quit", "General"],
];
const GROUPS = [...new Set(KEYS.map(k => k[2]))];

const MAP = {
  "": { o: ["open", 0, "o"], d: ["definition", 2, "d"], f12: ["definition", 2, "F12"], u: ["usages", 4, "u"], "[": ["back", 5, "["], "]": ["forward", 5, "]"],
    "/": ["find", 7, "/"], n: ["next", 8, "n"], s: ["search", 9, "s"], c: ["next hunk", 10, "c"], m: ["viewed", 11, "m"], return: ["edit", 13, "Enter"],
    v: ["select", 20, "v"], up: ["up", 28, "↑"], down: ["down", 28, "↓"], left: ["left", 28, "←"], right: ["right", 28, "→"], t: ["tree", 35, "t"],
    tab: ["focus", 36, "Tab"], w: ["wrap", 46, "w"], p: ["preview", 47, "p"], f: ["fold", 48, "f"], esc: ["close", 50, "Esc"], q: ["quit", 51, "q"] },
  shift: { d: ["symbols", 3, "D"], f12: ["usages", 4, "Shift+F12"], ";": ["go to line", 6, ":"], n: ["previous", 8, "N"], c: ["prev hunk", 10, "C"],
    up: ["select", 21, "Shift+↑"], down: ["select", 21, "Shift+↓"], left: ["select", 22, "Shift+←"], right: ["select", 22, "Shift+→"],
    "[": ["paragraph", 32, "{"], "]": ["paragraph", 32, "}"], "/": ["help", 44, "?"], t: ["theme", 49, "T"] },
  ctrl: { e: ["open", 0, "Ctrl+E"], n: ["new file", 1, "Ctrl+N"], g: ["go to line", 6, "Ctrl+G"], f: ["find", 7, "Ctrl+F"], s: ["save", 14, "Ctrl+S"],
    r: ["reload", 15, "Ctrl+R"], z: ["undo", 16, "Ctrl+Z"], y: ["redo", 16, "Ctrl+Y"], c: ["copy", 17, "Ctrl+C"], x: ["cut", 18, "Ctrl+X"],
    d: ["half down", 33, "Ctrl+D"], u: ["half up", 33, "Ctrl+U"] },
  alt: { left: ["word", 29, "Alt+←"], right: ["word", 29, "Alt+→"], delete: ["word", 19, "Alt+Backspace"] },
  fn: { left: ["home", 30, "Home"], right: ["end", 30, "End"], up: ["page", 34, "PgUp"], down: ["page", 34, "PgDn"], delete: ["delete", null, "Delete"] },
  "alt+shift": { left: ["word", 23, "Alt+Shift+←"], right: ["word", 23, "Alt+Shift+→"] },
  "ctrl+shift": { left: ["to start", 24, "Ctrl+Shift+←"], right: ["to end", 24, "Ctrl+Shift+→"] },
  "fn+shift": { left: ["to start", 25, "Shift+Home"], right: ["to end", 25, "Shift+End"], up: ["page", 26, "Shift+PgUp"], down: ["page", 26, "Shift+PgDn"] },
  "ctrl+fn": { left: ["file start", 31, "Ctrl+Home"], right: ["file end", 31, "Ctrl+End"] },
  "ctrl+fn+shift": { left: ["file start", 27, "Ctrl+Shift+Home"], right: ["file end", 27, "Ctrl+Shift+End"] },
};
const LAYERS = Object.keys(MAP);
const GLYPH = { shift: "⇧", ctrl: "⌃", alt: "⌥", fn: "fn" };
const prefix = layer => layer ? layer.split("+").map(m => GLYPH[m]).join("") + " " : "";
const SHIFTED = { "`": "~", 1: "!", 2: "@", 3: "#", 4: "$", 5: "%", 6: "^", 7: "&", 8: "*", 9: "(", 0: ")", "-": "_", "=": "+", "[": "{", "]": "}", "\\": "|", ";": ":", "'": '"', ",": "<", ".": ">", "/": "?" };

const R = (...a) => a;
const ROWS = [
  [R("esc", 1.5, "esc"), ...Array.from({ length: 12 }, (_, i) => R("f" + (i + 1), 1, "F" + (i + 1))), R("touch", 1, "")],
  ["`", ..."1234567890-=", R("delete", 1.5, "delete")],
  [R("tab", 1.5, "tab"), ..."qwertyuiop[]\\"],
  [R("caps", 1.75, "caps lock"), ..."asdfghjkl;'", R("return", 1.75, "return")],
  [R("shiftL", 2.25, "shift"), ..."zxcvbnm,./", R("shiftR", 2.25, "shift")],
  [R("fn", 1, "fn"), R("ctrl", 1, "control"), R("opt", 1, "option"), R("cmd", 1.25, "command"), R("space", 5, ""), R("cmdR", 1.25, "command"), R("optR", 1, "option"), "ARROWS"],
];
const MODS = { shiftL: "shift", shiftR: "shift", ctrl: "ctrl", ctrlR: "ctrl", opt: "alt", optR: "alt", fn: "fn" };
const MODGLYPH = { shiftL: "⇧", shiftR: "⇧", ctrl: "⌃", ctrlR: "⌃", opt: "⌥", optR: "⌥", cmd: "⌘", cmdR: "⌘", fn: "", caps: "⇪", tab: "⇥", return: "↩", delete: "⌫", esc: "" };
const CODE = { Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]", Backslash: "\\", Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
  Backspace: "delete", Enter: "return", NumpadEnter: "return", Tab: "tab", Escape: "esc", Space: "space", CapsLock: "caps",
  ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down", ShiftLeft: "shiftL", ShiftRight: "shiftR",
  ControlLeft: "ctrl", ControlRight: "ctrl", AltLeft: "opt", AltRight: "optR", MetaLeft: "cmd", MetaRight: "cmdR" };
const FNCODE = { Home: "left", End: "right", PageUp: "up", PageDown: "down", Delete: "delete" };

const root = document.getElementById("keys");
const host = root.querySelector("[data-kx-kb]"), out = root.querySelector("[data-kx-readout]"), groupsEl = root.querySelector("[data-kx-groups]");
const esc = s => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;");

const ARROW = { left: "◀", up: "▲", down: "▼", right: "▶" };
function cap(c) {
  const [id, w, name] = Array.isArray(c) ? c : [c, 1, null];
  const kind = name !== null ? (id.startsWith("f") && /^f\d/.test(id) ? " fk" : id === "esc" ? " fk esc" : id === "touch" ? " fk tid" : " named") : "";
  const glyph = MODGLYPH[id] ? `<i class="g">${MODGLYPH[id]}</i>` : "";
  const legend = ARROW[id] ? `<b class="ch">${ARROW[id]}</b>` : name !== null ? `<span class="nm">${name}</span>` : `<b class="ch"></b><i class="sh"></i>`;
  const icon = ICONS[id] ? `<i class="ic">${ICONS[id]}</i>` : "";
  return `<button type="button" tabindex="-1" class="kx${kind}${MODS[id] ? " mod" : ""}${SHIFTED[id] ? " dual" : ""}${ARROW[id] ? " ar" : ""}" style="--w:${w}" data-id="${esc(id)}">${icon}${glyph}${legend}<small class="wd"></small></button>`;
}
const arrows = `<div class="kx-arrows"><button type="button" tabindex="-1" class="kx ar" data-id="left"><b class="ch">◀</b><small class="wd"></small></button>` +
  `<div class="kx-ud"><button type="button" tabindex="-1" class="kx ar" data-id="up"><b class="ch">▲</b><small class="wd"></small></button><button type="button" tabindex="-1" class="kx ar" data-id="down"><b class="ch">▼</b><small class="wd"></small></button></div>` +
  `<button type="button" tabindex="-1" class="kx ar" data-id="right"><b class="ch">▶</b><small class="wd"></small></button></div>`;
host.innerHTML = `<div class="kx-board">${ROWS.map(r => `<div class="kx-row">${r.map(c => c === "ARROWS" ? arrows : cap(c)).join("")}</div>`).join("")}</div>`;
const caps = [...host.querySelectorAll(".kx")];
const look = (layer, id) => MAP[layer]?.[id];
const byId = id => caps.filter(c => c.dataset.id === id);

const IDS = Object.fromEntries(GROUPS.map(g => [g, new Set()]));
for (const l of LAYERS) for (const [id, e] of Object.entries(MAP[l])) if (e[1] !== null) IDS[KEYS[e[1]][2]].add(id);
function mini(g) {
  let y = 0, s = "";
  const rect = (x, y, w, h, id) => `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx=".8"${IDS[g].has(id) ? ' class="on"' : ""}/>`;
  ROWS.forEach((r, i) => {
    const h = i ? 6 : 4;
    let x = 0;
    for (const c of r) {
      if (c === "ARROWS") {
        s += rect(x, y + 3.25, 6, 2.75, "left") + rect(x + 7, y, 6, 2.75, "up") + rect(x + 7, y + 3.25, 6, 2.75, "down") + rect(x + 14, y + 3.25, 6, 2.75, "right");
        continue;
      }
      const [id, w] = Array.isArray(c) ? c : [c, 1];
      s += rect(x, y, w * 7 - 1, h, id);
      x += w * 7;
    }
    y += h + 1;
  });
  return `<svg class="kx-map" viewBox="0 0 101 39" aria-hidden="true">${s}</svg>`;
}
groupsEl.innerHTML = GROUPS.map(g => `<button type="button" data-g="${g}" aria-label="${g}">${mini(g)}<span>${g}</span></button>`).join("");

const held = new Set(), latched = new Set();
let group, pinned = null, shownLayer = null;
const shown = () => group !== undefined ? group : pinned;
const gOf = b => b.dataset.g || null;
const layerOf = set => {
  const k = [...set].sort().join("+");
  return k in MAP ? k : ["ctrl", "alt", "shift", "fn"].find(m => set.has(m)) || "";
};
const active = () => new Set([...held, ...latched]);

function render() {
  const layer = layerOf(active());
  const mods = active();
  const g = shown();
  for (const c of caps) {
    const id = c.dataset.id;
    const wd = c.querySelector(".wd"), ch = c.querySelector(".ch"), sh = c.querySelector(".sh");
    let entry = look(layer, id), from = layer;
    if (g) {
      entry = null;
      for (const l of [layer, ...LAYERS]) { const e = look(l, id); if (e && e[1] !== null && KEYS[e[1]][2] === g) { entry = e; from = l; break; } }
    }
    if (ch && !c.classList.contains("ar")) {
      const shiftOn = mods.has("shift") && !g;
      const upper = SHIFTED[id] ?? (id.length === 1 && /[a-z]/.test(id) ? id.toUpperCase() : "");
      ch.textContent = shiftOn && upper ? upper : id;
      if (sh) sh.textContent = SHIFTED[id] ? (shiftOn ? id : SHIFTED[id]) : "";
    }
    const arBase = c.classList.contains("ar") && !layer && !g;
    const word = entry && !arBase ? (g && from !== layer ? prefix(from) : "") + entry[0] : "";
    c.classList.toggle("worded", !!word);
    if (wd.textContent !== word) { wd.textContent = word; wd.classList.remove("in"); void wd.offsetWidth; wd.classList.add("in"); }
    c.classList.toggle("act", !!entry && entry[1] !== null);
    c.classList.toggle("dim", !!g && !entry && !c.classList.contains("mod"));
    const m = MODS[c.dataset.id];
    c.classList.toggle("held", !!m && mods.has(m));
    c.classList.toggle("lit", !!g && !!entry);
  }
  if (g) for (const l of LAYERS.filter(l => l && Object.values(MAP[l]).some(e => e[1] !== null && KEYS[e[1]][2] === g)))
    for (const m of l.split("+")) for (const c of caps) if (MODS[c.dataset.id] === m) c.classList.add("lit");
  drawState(g);
  if (shownLayer !== layer && !g) { shownLayer = layer; if ((!touched || layer) && Date.now() - saidAt > 2500) idle(); }
}

const comboHtml = s => s.split("+").map(k => `<kbd>${esc(k)}</kbd>`).join(`<span class="plus">+</span>`);
let saidAt = 0;
function say(entry, id) {
  saidAt = Date.now();
  if (!entry || entry[1] === null) {
    const name = id ? (id.length === 1 ? id : ({ shiftL: "shift", shiftR: "shift", optR: "option", opt: "option", cmdR: "command", cmd: "command", ctrl: "control" })[id] || id) : "";
    out.innerHTML = `<div class="ro-keys">${name ? `<kbd>${esc(name)}</kbd>` : ""}</div><div class="ro-text"><p class="ro-what dim">${MODS[id] ? "Hold it to see what the other keys do with it." : "merl does not use this key."}</p></div>`;
  } else {
    const k = KEYS[entry[1]];
    out.innerHTML = `<div class="ro-keys">${comboHtml(entry[2])}</div><div class="ro-text"><p class="ro-what">${esc(k[1])}</p><p class="ro-group">${k[2]}</p></div>`;
  }
  out.classList.remove("pop"); void out.offsetWidth; out.classList.add("pop");
}
function idle() {
  const layer = layerOf(active());
  if (layer) {
    const n = Object.values(MAP[layer]).filter(e => e[1] !== null).length;
    out.innerHTML = `<div class="ro-keys">${comboHtml(layer.split("+").map(m => ({ shift: "shift", ctrl: "control", alt: "option", fn: "fn" })[m]).join("+"))}</div><div class="ro-text"><p class="ro-what">${n} more ${n === 1 ? "key" : "keys"} with it. Press one.</p></div>`;
  } else out.innerHTML = demoing
    ? `<div class="ro-keys"><span class="ro-dot"></span></div><div class="ro-text"><p class="ro-what">Playing by itself. Press any key to take over.</p></div>`
    : `<div class="ro-keys"><span class="ro-dot"></span></div><div class="ro-text"><p class="ro-what">Press any key on your keyboard, or point at one.</p></div>`;
}
const timers = new Map();
function press(id) {
  for (const c of byId(id)) {
    c.classList.add("down");
    clearTimeout(timers.get(c));
    timers.set(c, setTimeout(() => c.classList.remove("down"), 180));
  }
}
function act(id) {
  press(id);
  const g = shown();
  const layer = layerOf(active());
  let entry = look(layer, id);
  if (g && !entry) for (const l of LAYERS) { const e = look(l, id); if (e && e[1] !== null && KEYS[e[1]][2] === g) { entry = e; break; } }
  say(entry, id);
}

let touched = false, demoing = false, demoRun = 0;
function stop() { demoRun++; demoing = false; }
function take() { if (touched) return; touched = true; stop(); latched.clear(); held.clear(); render(); }

addEventListener("keydown", e => {
  if (e.target.closest?.("[data-screen], input, textarea, [contenteditable]") || e.metaKey) return;
  const rect = host.getBoundingClientRect();
  if (rect.bottom < 0 || rect.top > innerHeight || !rect.height) return;
  take();
  held.clear();
  if (e.shiftKey) held.add("shift");
  if (e.ctrlKey) held.add("ctrl");
  if (e.altKey) held.add("alt");
  let id = CODE[e.code] || (/^Key[A-Z]$/.test(e.code) ? e.code[3].toLowerCase() : /^Digit\d$/.test(e.code) ? e.code[5] : /^F\d+$/.test(e.code) ? e.code.toLowerCase() : null);
  if (FNCODE[e.code]) { id = FNCODE[e.code]; held.add("fn"); press("fn"); }
  render();
  if (!id) return;
  if (MODS[id]) { press(id); return; }
  if (e.key === "/" || e.key === "'") e.preventDefault();
  act(id);
});
addEventListener("keyup", e => {
  if (!touched) return;
  held.clear();
  if (e.shiftKey) held.add("shift");
  if (e.ctrlKey) held.add("ctrl");
  if (e.altKey) held.add("alt");
  render();
});
addEventListener("blur", () => { held.clear(); render(); });

host.addEventListener("pointerover", e => {
  const c = e.target.closest(".kx");
  if (!c || MODS[c.dataset.id]) return;
  take();
  const g = shown(), layer = layerOf(active());
  let entry = look(layer, c.dataset.id);
  if (g) { entry = null; for (const l of [layer, ...LAYERS]) { const x = look(l, c.dataset.id); if (x && x[1] !== null && KEYS[x[1]][2] === g) { entry = x; break; } } }
  if (entry) say(entry, c.dataset.id);
});
host.addEventListener("click", e => {
  const c = e.target.closest(".kx");
  if (!c) return;
  take();
  const m = MODS[c.dataset.id];
  if (m) { latched.has(m) ? latched.delete(m) : latched.add(m); press(c.dataset.id); render(); idle(); return; }
  act(c.dataset.id);
  if (latched.size) { latched.clear(); setTimeout(render, 220); }
});
const tell = () => { const g = shown(); g ? sayGroup(g) : idle(); };
function preview(g) { take(); if (group === g) return; group = g; render(); tell(); }
function unpreview() { if (group === undefined) return; group = undefined; render(); tell(); }
function pin(g) { take(); pinned = g; group = undefined; render(); tell(); }
function drawState(g) {
  for (const b of groupsEl.querySelectorAll("[data-g]")) {
    b.setAttribute("aria-pressed", gOf(b) === pinned);
    b.classList.toggle("on", gOf(b) === g);
  }
}
groupsEl.addEventListener("pointerover", e => { const b = e.target.closest("[data-g]"); if (b) preview(gOf(b)); });
groupsEl.addEventListener("pointerleave", unpreview);
groupsEl.addEventListener("focusin", e => { const b = e.target.closest("[data-g]"); if (b && b.matches(":focus-visible")) preview(gOf(b)); });
groupsEl.addEventListener("focusout", e => { if (groupsEl.contains(e.relatedTarget)) return; if (!groupsEl.matches(":hover")) unpreview(); });
groupsEl.addEventListener("click", e => {
  const b = e.target.closest("[data-g]");
  if (!b) return;
  const g = gOf(b);
  pin(g && pinned === g ? null : g);
});
function hold(g) {
  const names = { shift: "⇧ is shift", ctrl: "⌃ is control", alt: "⌥ is option", fn: "fn is fn" };
  const used = new Set();
  for (const l of LAYERS.slice(1)) for (const e of Object.values(MAP[l])) if (e[1] !== null && KEYS[e[1]][2] === g) l.split("+").forEach(m => used.add(m));
  return used.size ? ` Hold the lit modifier: ${[...used].map(m => names[m]).join(", ")}.` : "";
}
function sayGroup(g) {
  const n = caps.filter(c => c.classList.contains("lit") && !MODS[c.dataset.id]).length;
  out.innerHTML = `<div class="ro-keys"><span class="ro-g">${n}</span></div><div class="ro-text"><p class="ro-what">${n} keys for ${g.toLowerCase()}.${hold(g)}</p><p class="ro-group">Point at a lit key for what it does</p></div>`;
}

const DEMO = [["", "o"], ["", "d"], ["", "u"], ["", "["], ["shift", "d"], ["", "c"], ["", "m"], ["ctrl", "s"], ["alt", "right"], ["", "v"], ["shift", "/"], ["fn", "down"], ["", "t"], ["", "q"]];
const sleep = ms => new Promise(r => setTimeout(r, ms));
async function demo() {
  const my = ++demoRun;
  demoing = true;
  idle();
  await sleep(1400);
  for (let i = 0; my === demoRun; i = (i + 1) % DEMO.length) {
    const [layer, id] = DEMO[i];
    latched.clear();
    if (layer) { latched.add(layer); render(); for (const c of caps) if (MODS[c.dataset.id] === layer) press(c.dataset.id); await sleep(650); if (my !== demoRun) return; }
    act(id);
    await sleep(1900);
    if (my !== demoRun) return;
    if (layer) { latched.clear(); render(); }
  }
}
new IntersectionObserver(es => {
  const on = es[0].isIntersecting;
  if (on && !touched && !demoing) demo();
  if (!on && demoing) { stop(); latched.clear(); render(); idle(); }
}, { threshold: 0.55 }).observe(host);

root.querySelector("[data-kx-list]").innerHTML = GROUPS.map((g, i) => `<details class="kx-lg"${i ? "" : " open"}><summary>${g}<span>${KEYS.filter(k => k[2] === g).length}</span></summary><dl>${KEYS.filter(k => k[2] === g).map(([ks, w]) =>
  `<div><dt>${ks.split(" / ").map(k => `<kbd>${esc(k)}</kbd>`).join(" ")}</dt><dd>${esc(w)}</dd></div>`).join("")}</dl></details>`).join("");

render();
idle();
