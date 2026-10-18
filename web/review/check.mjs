import fs from "node:fs";

const [wasmPath, bundlePath, flag] = process.argv.slice(2);
if (!bundlePath) {
  console.error("usage: node web/review/check.mjs MERL.wasm REVIEW.json [--write]\n\n" +
    "Plays each language's steps on the wasm build, as the site does, and fails on a step that does not\n" +
    "end where it wants. --write fills the answers steps.json leaves to the run into REVIEW.json.\n" +
    "The WASI shim is the site's own (web/site/wasi), or the module WASI_SHIM names.");
  process.exit(2);
}
const shim = process.env.WASI_SHIM ?? new URL("../site/wasi/index.js", import.meta.url).href;
const { WASI, File, Directory, PreopenDirectory, ConsoleStdout } = await import(shim);
const B = JSON.parse(fs.readFileSync(bundlePath, "utf8"));
const enc = new TextEncoder(), dec = new TextDecoder();
const NAMED = ["Backspace", "Enter", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End",
  "PageUp", "PageDown", "Tab", "BackTab", "Delete", "Insert", "Escape"];
const sleep = ms => new Promise(r => setTimeout(r, ms));
const module = await WebAssembly.compile(fs.readFileSync(wasmPath));

const tree = files => {
  const top = new Map();
  for (const [p, t] of Object.entries(files)) {
    const parts = p.split("/");
    let d = top;
    for (const part of parts.slice(0, -1)) {
      if (!d.has(part)) d.set(part, new Directory(new Map()));
      d = d.get(part).contents;
    }
    d.set(parts.at(-1), new File(enc.encode(t)));
  }
  return top;
};

const ok = (w, s) => (w.mode ? s.mode.startsWith(w.mode) : !/^(Picker|Find|Search)/.test(s.mode))
  && (!w.file || s.file === w.file) && (!w.line || s.line === w.line)
  && (w.renamed === undefined || s.review?.renamed === w.renamed)
  && (!w.viewed || s.review?.viewed >= w.viewed);

let bad = 0;
const table = [];
for (const [lang, L] of Object.entries(B.langs)) {
  const fsRoot = new Map([
    [`review-${lang}`, new Directory(new Map([[B.slug, new Directory(tree({ ...L.files, ".git/merl-replay": L.replay }))]]))],
    ["tmp", new Directory(new Map())],
  ]);
  const log = ConsoleStdout.lineBuffered(m => console.log("[merl]", m));
  const wasi = new WASI([], ["HOME=/home"], [new File([]), log, log, new PreopenDirectory("/", fsRoot)], { debug: false });
  const inst = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
  wasi.initialize(inst);
  const ex = inst.exports;
  const read = p => dec.decode(new Uint8Array(ex.memory.buffer, p, ex.merl_out_len()));
  const call = (fn, s) => {
    const b = enc.encode(s);
    new Uint8Array(ex.memory.buffer, ex.merl_buf(b.length), b.length).set(b);
    return read(fn());
  };
  const err = call(ex.merl_review, `/review-${lang}/${B.slug}\n${B.base}\n110\n30`);
  if (err) {
    console.log(lang, "open failed:", err);
    bad++;
    table.push([lang, "open failed"]);
    continue;
  }
  const state = () => JSON.parse(read(ex.merl_state()));
  const paint = () => JSON.parse(read(ex.merl_frame()));
  const key = k => {
    const n = NAMED.indexOf(k);
    ex.merl_key(n >= 0 ? 0x110000 + n : k.codePointAt(0), n < 0 && k !== k.toLowerCase() ? 1 : 0);
    paint();
  };
  const press = async k => {
    if (k.length > 1 && !NAMED.includes(k)) {
      for (const c of k) {
        key(c);
        await sleep(10);
      }
    } else key(k);
    for (let i = 0; i < 16; i++) {
      if (ex.merl_tick()) paint();
      await sleep(20);
    }
  };
  paint();
  console.log(`== ${lang}: starts at ${JSON.stringify(state()).slice(0, 120)}`);
  const row = [lang];
  for (const [i, st] of L.steps.entries()) {
    const before = state();
    if (st.want === "auto") {
      for (const k of st.answer) await press(k);
      const s = state();
      st.want = { file: s.file, line: s.line };
    } else {
      if (st.want.viewed === "all") st.want.viewed = before.review.files;
      if (st.answer === "until") {
        st.answer = [];
        while (!ok(st.want, state()) && st.answer.length < 20) {
          st.answer.push(st.keys[0]);
          await press(st.keys[0]);
        }
      } else for (const k of st.answer) await press(k);
    }
    const s = state(), good = ok(st.want, s);
    if (!good) bad++;
    row.push(good ? "pass" : "FAIL");
    console.log(`${i + 1} ${good ? "ok  " : "FAIL"} ${JSON.stringify(st.answer)} -> ${s.file}:${s.line} ${s.mode} ${JSON.stringify(s.review)} ${s.message}`);
  }
  table.push(row);
  const misses = read(ex.merl_git_misses());
  if (misses) console.log(`${lang}: ${misses.split("\n").length} git calls not in the replay:\n${misses.slice(0, 600)}`);
}
console.log(`\nlanguage   ${B.langs[Object.keys(B.langs)[0]].steps.map((_, i) => `step ${i + 1}`).join("  ")}`);
for (const [lang, ...cells] of table) console.log(lang.padEnd(10), cells.map(c => c.padEnd(6)).join(" "));
if (flag === "--write") fs.writeFileSync(bundlePath, JSON.stringify(B));
console.log(bad ? `${bad} steps failed` : "every step passed");
process.exit(bad ? 1 : 0);
