import { play } from "./shared/play.js";

const html = document.documentElement;
html.classList.add("js");
const $ = s => document.querySelector(s);
const sleep = ms => new Promise(r => setTimeout(r, ms));

await Promise.all([...document.querySelectorAll("[data-include]")].map(async el => {
  const r = await fetch(el.dataset.include);
  if (!r.ok) return el.remove();
  el.outerHTML = await r.text();
}));

$("[data-mode]").onclick = () => {
  html.dataset.theme = html.dataset.theme === "dark" ? "light" : "dark";
  try { localStorage.setItem("merl-theme", html.dataset.theme); } catch {}
  dispatchEvent(new Event("merl:theme"));
};

for (const b of document.querySelectorAll("[data-copy]")) b.addEventListener("click", async () => {
  const em = b.querySelector("em");
  try { await navigator.clipboard.writeText(b.dataset.copy); if (em) em.textContent = "Copied"; }
  catch { getSelection().selectAllChildren(b.querySelector("code") || b); if (em) em.textContent = "Selected"; }
  b.classList.add("copied");
  setTimeout(() => { if (em) em.textContent = "Copy"; b.classList.remove("copied"); }, 1600);
});
const io = new IntersectionObserver(es => es.forEach(e => e.isIntersecting && (e.target.classList.add("in"), io.unobserve(e.target))), { threshold: 0.12, rootMargin: "0px 0px -6% 0px" });
document.querySelectorAll(".reveal").forEach(el => io.observe(el));

for (const name of ["hero", "keys", "work", "not", "why"]) import(`./parts/${name}.js`).catch(e => console.error(name, e));

const stage = $("[data-play]"), coachEl = $("[data-task-coach]"), touchEl = $("[data-touch]");
const edTheme = () => html.dataset.theme === "dark" ? "tokyonight-moon" : "vscode-light";
const fitHeight = () => {
  const top = stage.querySelector("[data-screen]").getBoundingClientRect().top + scrollY;
  return Math.max(330, Math.min(680, innerHeight - top - 36));
};
const review = await fetch("review.json").then(r => r.json());
$("[data-pr-title]").textContent = review.title;
$("[data-pr-meta]").innerHTML = `<b>${review.author}</b> wants to merge 1 commit into <code>${review.base}</code> from <code>${review.branch}</code>`;

let last = null, sayTimer = 0, watching = 0;
const keyCap = k => ({ Escape: "Esc", Enter: "Enter", ArrowDown: "↓", ArrowUp: "↑" })[k] || k;
function renderTask(st) {
  last = st;
  const { step, steps, said, done } = st, n = steps.length, cur = steps[Math.min(step, n - 1)];
  const saidHtml = said ? `<p class="tk-said">${said}</p>` : "";
  const shown = cur.answer.length > 1 && cur.answer.every(k => k === cur.answer[0]) ? cur.keys : cur.answer;
  const keys = done ? "" : `<div class="tk-keys">${shown.map(k => `<kbd>${keyCap(k)}</kbd>`).join("")}</div>`;
  const dots = `<ol class="tk-dots">${steps.map((_, i) => `<li class="${i < step ? "done" : i === step ? "cur" : ""}"></li>`).join("")}</ol>`;
  const show = (done ? `<button type="button" class="tk-btn" data-again>Start again</button>` : `<button type="button" class="tk-btn" data-show>Show me</button>`)
    + `<button type="button" class="tk-btn take" data-take>Try it yourself</button>`;
  const num = done ? `<div class="tk-n done"><b>✓</b><span>done</span></div>` : `<div class="tk-n"><b>${step + 1}</b><span>of ${n}</span></div>`;
  coachEl.innerHTML = `${num}<div class="tk-body">${saidHtml}<p class="tk-ask">${done ? review.done : cur.ask}</p>${keys}</div><div class="tk-foot">${dots}${show}</div>`;
  coachEl.classList.toggle("is-said", !!said);
  coachEl.classList.remove("swap"); void coachEl.offsetWidth; coachEl.classList.add("swap");
  touchEl.innerHTML = done ? "" : cur.keys.map(k => `<button type="button" data-k="${k}">${keyCap(k)}</button>`).join("");
  if (said && !done) { clearTimeout(sayTimer); sayTimer = setTimeout(() => renderTask({ ...last, said: null }), 4200); }
}
const typing = () => 55 + Math.random() * 95;
async function playKeys(keys, my) {
  for (const k of keys) {
    if (my !== watching) return false;
    if (k.length > 1 && !["Escape", "Enter", "ArrowDown", "ArrowUp"].includes(k)) {
      for (const c of k) { merl.press(c, 0); await sleep(typing()); }
    } else merl.press(k, 0);
    await sleep(k === "c" ? 650 : 420);
  }
  return true;
}
async function playStep(i, my) {
  const ans = merl.steps()[i].answer, until = ans.length > 1 && ans.every(k => k === ans[0]);
  if (!(await playKeys(ans, my))) return false;
  for (let n = 0; until && merl.step() === i && n < 12; n++) if (!(await playKeys([ans[0]], my))) return false;
  return true;
}
async function show() {
  const my = ++watching, st = merl.steps()[merl.step()];
  if (!st) return;
  stage.classList.add("showing");
  await playStep(merl.step(), my);
  if (my === watching) stage.classList.remove("showing");
}
async function watchAll() {
  const my = ++watching;
  stage.classList.add("showing", "demo");
  for (let i = merl.step(); i < merl.steps().length; i++) {
    await sleep(i ? 2600 : 900);
    if (my !== watching || !(await playStep(i, my))) return;
  }
  await sleep(3200);
  if (my !== watching) return;
  stage.classList.remove("showing", "demo");
  merl.open(merl.project());
}
const stopWatching = () => { watching++; stage.classList.remove("showing", "demo"); };
const takeOver = () => { stopWatching(); merl.open(merl.project()); stage.querySelector("[data-screen]").focus({ preventScroll: true }); };
stage.addEventListener("click", e => {
  if (e.target.closest("[data-take]")) takeOver();
  else if (e.target.closest("[data-show]")) { show(); stage.querySelector("[data-screen]").focus({ preventScroll: true }); }
  else if (e.target.closest("[data-again]")) { stopWatching(); merl.open(merl.project()); stage.querySelector("[data-screen]").focus({ preventScroll: true }); }
  else if (e.target.closest("[data-k]")) merl.press(e.target.closest("[data-k]").dataset.k, 0);
});
addEventListener("keydown", e => {
  if (!e.isTrusted || e.metaKey || e.ctrlKey || e.altKey) return;
  const word = e.key.length === 1 && e.key !== " " || e.key === "Escape" || e.key === "Enter";
  if (stage.classList.contains("demo") && (stage.contains(e.target) || word && e.target === document.body && stageSeen)) { e.preventDefault(); e.stopPropagation(); takeOver(); }
  else if (stage.classList.contains("showing") && stage.contains(e.target)) stopWatching();
}, true);
stage.addEventListener("pointerdown", e => {
  if (e.target.closest("[data-take], .tk-coach")) return;
  if (stage.classList.contains("demo") && e.target.closest("[data-screen]")) takeOver();
  else if (stage.classList.contains("showing") && e.target.closest("[data-screen]")) stopWatching();
}, true);
let stageSeen = true;
new IntersectionObserver(([e]) => stageSeen = e.isIntersecting, { threshold: .4 }).observe(stage);

const merl = await play(stage, { height: fitHeight, theme: edTheme(), cols: 118, minRows: 18, maxRows: 30, review, onStatus: renderTask });
addEventListener("merl:theme", () => merl.theme(edTheme()));
window.merlSite = merl;
watchAll();
