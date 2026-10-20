import { NOT } from "../shared/content.js";

const clamp = x => Math.max(0, Math.min(1, x));
const lines = NOT.map(t => t.replace("keys are below", "keys are above"));

const rows = document.querySelector("[data-not-rows]"), count = document.querySelector("[data-not-count]"), win = document.querySelector("[data-not-p]");
rows.innerHTML = lines.map(t => `<div class="not-row"><span>${t}</span></div>`).join("");

function frame() {
  const vh = innerHeight, r = win.getBoundingClientRect(), p = clamp((vh * 0.9 - r.top) / (vh * 0.5));
  const n = Math.round(p * lines.length);
  [...rows.children].forEach((el, i) => el.classList.toggle("gone", i < n));
  count.textContent = n;
}
let queued = false;
const tick = () => { if (!queued) { queued = true; requestAnimationFrame(() => { queued = false; frame(); }); } };
addEventListener("scroll", tick, { passive: true });
addEventListener("resize", tick);
frame();
