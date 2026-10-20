import { WASI, File, Directory, PreopenDirectory, ConsoleStdout } from "../wasi/index.js";

const LANGS = [["python", "Python"], ["typescript", "TypeScript"], ["go", "Go"], ["rust", "Rust"]];
const FG = "#c8d3f5", BG = "#222436", FONT = '"SF Mono", SFMono-Regular, ui-monospace, Menlo, "DejaVu Sans Mono", monospace';
const ANSI = "1b1d2b ff757f c3e88d ffc777 82aaff c099ff 86e1fc 828bb8 444a73 ff757f c3e88d ffc777 82aaff c099ff 86e1fc c8d3f5".split(" ").map(h => "#" + h);
export const NAMED = ["Backspace", "Enter", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "PageUp", "PageDown", "Tab", "BackTab", "Delete", "Insert", "Escape"];
const CAPS = { ArrowLeft: "←", ArrowRight: "→", ArrowUp: "↑", ArrowDown: "↓", Escape: "Esc", Backspace: "⌫", " ": "Space", BackTab: "⇧Tab" };
const SEG = { "│": "ud", "┃": "ud", "─": "lr", "━": "lr", "┌": "dr", "┐": "dl", "└": "ur", "┘": "ul",
  "├": "udr", "┤": "udl", "┬": "dlr", "┴": "ulr", "┼": "udlr", "╭": "dr", "╮": "dl", "╰": "ur", "╯": "ul" };
const enc = new TextEncoder(), dec = new TextDecoder();

export async function play(root, opts = {}) {
  const q = s => root.querySelector(`[data-${s}]`);
  const screen = q("screen"), canvas = screen.querySelector("canvas"), cursorEl = q("cursor"), keycap = q("keycap"),
    langs = q("lang"), where = q("where");
  const maxCols = opts.cols || 112, minRows = opts.minRows || 18, maxRows = opts.maxRows || 34;

  const bytes = await fetch(new URL("../merl.wasm.gz", import.meta.url)).then(r => new Response(r.body.pipeThrough(new DecompressionStream("gzip"))).arrayBuffer());
  const review = opts.review;
  const tree = files => {
    const top = new Map();
    for (const [p, text] of Object.entries(files)) {
      const parts = p.split("/");
      let d = top;
      for (const part of parts.slice(0, -1)) {
        if (!d.has(part)) d.set(part, new Directory(new Map()));
        d = d.get(part).contents;
      }
      d.set(parts.at(-1), new File(enc.encode(text)));
    }
    return new Directory(top);
  };
  const fsRoot = new Map();
  for (const [lang, r] of Object.entries(review.langs))
    fsRoot.set(`review-${lang}`, new Directory(new Map([[review.slug, tree({ ...r.files, ".git/merl-replay": r.replay })]])));
  fsRoot.set("tmp", new Directory(new Map()));
  const log = ConsoleStdout.lineBuffered(m => console.log("[merl]", m));
  const wasi = new WASI([], [`MERL_UTC_OFFSET=${-new Date().getTimezoneOffset() * 60}`], [new File([]), log, log, new PreopenDirectory("/", fsRoot)], { debug: false });
  const inst = await WebAssembly.instantiate(await WebAssembly.compile(bytes), { wasi_snapshot_preview1: wasi.wasiImport });
  wasi.initialize(inst);
  const ex = inst.exports;
  const read = p => dec.decode(new Uint8Array(ex.memory.buffer, p, ex.merl_out_len()));
  const call = (fn, s) => {
    const b = enc.encode(s);
    const at = ex.merl_buf(b.length);
    new Uint8Array(ex.memory.buffer, at, b.length).set(b);
    return read(fn());
  };

  let lastBg = "", themeName = opts.theme || "", slug = review.slug, project = null, step = 0, size = 15, cell = { w: 9, h: 19 }, grid = [110, 30], pad = [0, 0], capTimer = 0;

  function colour(c, dflt) {
    if (!c) return dflt;
    if (c[0] === "#") return c;
    if (c[0] === "i") { const n = +c.slice(1); return n < 16 ? ANSI[n] : dflt; }
    const i = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "gray"].indexOf(c.replace("light", ""));
    return i < 0 ? dflt : ANSI[i + (c.startsWith("light") ? 8 : 0)];
  }

  function paint() {
    const f = JSON.parse(read(ex.merl_frame()));
    if (!f || f.error) return;
    const FGt = f.fg || FG, BGt = f.bg || BG;
    if (BGt !== lastBg) { lastBg = BGt; root.style.setProperty("--term", BGt); root.style.setProperty("--term-fg", FGt); root.style.setProperty("--term-accent", f.accent || FGt); }
    const dpr = devicePixelRatio || 1, ctx = canvas.getContext("2d");
    canvas.width = Math.ceil(f.w * cell.w * dpr); canvas.height = Math.ceil(f.h * cell.h * dpr);
    canvas.style.width = f.w * cell.w + "px"; canvas.style.height = f.h * cell.h + "px";
    ctx.scale(dpr, dpr); ctx.textBaseline = "middle";
    f.rows.forEach((row, y) => {
      let x = 0;
      for (const [text, fg0, bg0, m] of row) {
        let fg = colour(fg0, FGt), bg = colour(bg0, BGt);
        if (m & 64) [fg, bg] = [bg, fg];
        const chars = [...text];
        ctx.fillStyle = bg; ctx.fillRect(x * cell.w, y * cell.h, chars.length * cell.w + 0.5, cell.h);
        ctx.font = `${m & 4 ? "italic " : ""}${m & 1 ? "700 " : ""}${size}px ${FONT}`;
        ctx.globalAlpha = m & 2 ? 0.6 : 1;
        ctx.fillStyle = fg; ctx.strokeStyle = fg; ctx.lineWidth = 1;
        for (const c of chars) {
          const px = x * cell.w, py = y * cell.h, seg = SEG[c];
          if (seg) {
            const mx = Math.round(px + cell.w / 2) + 0.5, my = Math.round(py + cell.h / 2) + 0.5;
            ctx.beginPath();
            if (seg.includes("u")) { ctx.moveTo(mx, py); ctx.lineTo(mx, my); }
            if (seg.includes("d")) { ctx.moveTo(mx, my); ctx.lineTo(mx, py + cell.h); }
            if (seg.includes("l")) { ctx.moveTo(px, my); ctx.lineTo(mx, my); }
            if (seg.includes("r")) { ctx.moveTo(mx, my); ctx.lineTo(px + cell.w, my); }
            ctx.stroke();
          } else {
            const n = c.codePointAt(0) - 0x2580;
            if (n >= 1 && n <= 8) ctx.fillRect(px, py + cell.h * (1 - n / 8), cell.w, cell.h * n / 8);
            else if (n >= 9 && n <= 15) ctx.fillRect(px, py, cell.w * (16 - n) / 8, cell.h);
            else if (c !== " ") ctx.fillText(c, px, py + cell.h / 2 + 1);
          }
          if (m & 8) ctx.fillRect(px, py + cell.h - 2, cell.w, 1);
          x++;
        }
        ctx.globalAlpha = 1;
      }
    });
    cursorEl.hidden = !f.cursor;
    if (f.cursor) Object.assign(cursorEl.style, { left: pad[0] + f.cursor[0] * cell.w + "px", top: pad[1] + f.cursor[1] * cell.h + "px", width: cell.w + "px", height: cell.h + "px" });
  }

  function fit() {
    const cs = getComputedStyle(screen);
    pad = [parseFloat(cs.paddingLeft), parseFloat(cs.paddingTop)];
    const avail = screen.clientWidth - pad[0] * 2;
    const ctx = document.createElement("canvas").getContext("2d");
    ctx.font = `100px ${FONT}`;
    const ratio = ctx.measureText("M").width / 100;
    size = Math.max(9.5, Math.min(opts.maxFont || 15, avail / maxCols / ratio));
    cell = { w: size * ratio, h: Math.round(size * 1.25) };
    const budget = opts.height ? opts.height() : innerHeight * 0.62;
    const rows = Math.max(minRows, Math.min(maxRows, Math.floor(budget / cell.h)));
    grid = [Math.floor(avail / cell.w), rows];
    screen.style.height = rows * cell.h + pad[1] * 2 + "px";
    ex.merl_resize(grid[0], grid[1]);
  }

  const state = () => JSON.parse(read(ex.merl_state()));

  function check() {
    const steps = project.steps;
    if (step >= steps.length) return;
    const s = state(), want = steps[step].want || steps[step], said = steps[step].say;
    if (want.mode ? !s.mode.startsWith(want.mode) : /^(Picker|Find|Search)/.test(s.mode)) return;
    const ok = (!want.file || s.file === want.file) && (!want.line || s.line === want.line)
      && (!want.text || new RegExp(want.text, "i").test(s.text)) && (want.renamed === undefined || s.review?.renamed === want.renamed)
      && (!want.viewed || s.review?.viewed >= want.viewed) && (!want.message || s.message.includes(want.message));
    if (!ok) return;
    step++;
    status(said);
  }

  function status(said) {
    const steps = project.steps;
    opts.onStatus({ step, steps, said, done: step >= steps.length });
  }

  function open(name) {
    project = { name, steps: review.langs[name].steps }; step = 0;
    if (langs) [...langs.children].forEach(t => t.setAttribute("aria-pressed", t.dataset.name === name));
    if (where) where.textContent = `~/${slug} — merl --review`;
    fit();
    const err = call(ex.merl_review, `/review-${name}/${slug}\n${review.base}\n${grid[0]}\n${grid[1]}`);
    if (err) console.error(err);
    if (themeName) call(ex.merl_theme, themeName);
    if (grid[0] < 80) ex.merl_key("t".codePointAt(0), 0);
    paint(); status();
  }

  function showCap(k, mods) {
    if (!keycap) return;
    keycap.textContent = (mods & 2 ? "Ctrl+" : "") + (mods & 4 ? "Alt+" : "") + (CAPS[k] || (mods & 6 ? k.toUpperCase() : k));
    keycap.classList.remove("on"); void keycap.offsetWidth; keycap.classList.add("on");
    clearTimeout(capTimer); capTimer = setTimeout(() => keycap.classList.remove("on"), 900);
  }

  function press(key, mods) {
    const named = NAMED.indexOf(key);
    if (named >= 0) ex.merl_key(0x110000 + named, mods);
    else ex.merl_key(key.codePointAt(0), mods | (key !== key.toLowerCase() ? 1 : 0));
    showCap(key, mods);
    paint(); check();
  }

  screen.addEventListener("keydown", e => {
    if (e.metaKey) return;
    let k = e.key, mods = (e.ctrlKey ? 2 : 0) | (e.altKey ? 4 : 0);
    if (k === "Tab" && e.shiftKey) k = "BackTab";
    if (NAMED.includes(k)) mods |= e.shiftKey ? 1 : 0;
    else if (e.altKey && /^Key[A-Z]$/.test(e.code)) k = e.shiftKey ? e.code[3] : e.code[3].toLowerCase();
    else if ([...k].length !== 1) return;
    e.preventDefault();
    press(k, mods);
  });
  setInterval(() => { if (ex.merl_tick()) { paint(); check(); } }, 30);
  let lastW = 0;
  new ResizeObserver(() => { if (project && Math.abs(screen.clientWidth - lastW) > 1) { lastW = screen.clientWidth; fit(); paint(); } }).observe(screen);

  if (langs) LANGS.forEach(([name, label]) => {
    const b = document.createElement("button");
    b.type = "button"; b.dataset.name = name; b.textContent = label;
    b.onclick = () => { open(name); screen.focus({ preventScroll: true }); };
    langs.append(b);
  });
  open(LANGS[0][0]);
  q("loading")?.remove();
  const theme = name => { themeName = name; call(ex.merl_theme, name); paint(); };
  return { state, press, open, theme, step: () => step, steps: () => project.steps, project: () => project?.name };
}
