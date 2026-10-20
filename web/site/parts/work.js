import { KEYS } from "../shared/content.js";

const W = Object.fromEntries(KEYS.flatMap(([ks, w]) => ks.map(k => [k, w])));
const CUES = [[0, "", "A checkout of gitea, 5,500 files. Nothing was indexed or configured first."], [1.51, "o", W.o], [8.56, "/", W["/"]], [13.8, "d", W.d], [16.75, "[", W["["]], [18.82, "u", W.u], [25.34, "s", W.s]];
const html = document.documentElement;
const DIRS = { dark: "film", light: "film-light" };
const BG = { dark: ["#222436", "#c8d3f5"], light: ["#ffffff", "#343434"] };
const META = Object.fromEntries(await Promise.all(Object.values(DIRS).map(async d => [d, await fetch(new URL(`../${d}/film.json`, import.meta.url)).then(r => r.json())])));
const look = () => html.dataset.theme === "light" ? "light" : "dark";
const film = () => META[DIRS[look()]].demo;
const cueAt = c => film().cues?.[c] ?? CUES[c][0];

const el = document.querySelector("[data-wk]"), win = el.querySelector(".wk-win"), canvas = el.querySelector("canvas"), ctx = canvas.getContext("2d"),
  line = el.querySelector(".wk-line"), key = el.querySelector(".wk-key"), say = el.querySelector(".wk-say"), track = el.querySelector(".wk-track");
const cache = {};
const ok = im => im.complete && im.naturalWidth > 0;
function frames() {
  const dir = DIRS[look()];
  return cache[dir] ||= film().frames.map((_, i) => {
    const im = new Image();
    im.decoding = "async";
    im.onload = paint;
    im.src = new URL(`../${dir}/demo/${String(i).padStart(3, "0")}.webp`, import.meta.url);
    return im;
  });
}
const keyHtml = k => k ? `<kbd>${k}</kbd>` : "";

let t = 0, seen = false, paused = false, shown = -1, cue = -1, painted;
const total = () => film().total;
function paint() {
  const list = frames(), fr = film().frames;
  if (painted !== list) { painted = list; shown = -1; const [bg, fg] = BG[look()]; win.style.cssText = `--term:${bg}; --term-fg:${fg}`; }
  let i = fr.length - 1;
  while (i > 0 && fr[i][0] * 1000 > t) i--;
  while (i > 0 && !ok(list[i])) i--;
  if (ok(list[i]) && i !== shown) { ctx.drawImage(list[i], 0, 0, canvas.width, canvas.height); shown = i; }
  const c = CUES.findLastIndex((_, i) => cueAt(i) * 1000 <= t);
  if (c !== cue) {
    cue = c;
    key.innerHTML = keyHtml(CUES[c][1]);
    say.textContent = CUES[c][2];
    line.classList.remove("hit"); void el.offsetWidth; line.classList.add("hit");
  }
  track.style.setProperty("--p", (t / total()).toFixed(4));
}
track.addEventListener("pointerdown", e => {
  const r = track.getBoundingClientRect();
  t = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * total();
  paint();
});
win.addEventListener("click", () => paused = !paused);
new IntersectionObserver(es => es.forEach(e => { if (e.isIntersecting) frames(); }), { rootMargin: "150% 0px" }).observe(el.closest("section"));
new IntersectionObserver(es => es.forEach(e => { seen = e.isIntersecting; }), { threshold: 0.3 }).observe(el);
const restart = () => { t = 0; paint(); };
addEventListener("merl:theme", paint);
paint();

let last = performance.now();
function loop(ts) {
  const dt = Math.min(100, ts - last);
  last = ts;
  el.classList.toggle("paused", paused);
  if (seen && !paused && !document.hidden) {
    t += dt;
    if (t > total() + 1600) document.startViewTransition ? document.startViewTransition(restart) : restart();
    else { const now = t; t = Math.min(now, total()); paint(); t = now; }
  }
  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);
