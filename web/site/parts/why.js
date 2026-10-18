import { STATS } from "../shared/count.js";

const CELLS = [
  [STATS.commits, "commits on master"],
  [STATS.prs, "pull requests merged"],
  [STATS.releases, "releases"],
  [STATS.tests, "tests"],
  [STATS.themes, "themes"],
  [STATS.since, "first commit"],
];
const fmt = v => typeof v === "number" ? v.toLocaleString("en") : v;
const box = document.querySelector("[data-slots]");
box.innerHTML = CELLS.map(([v, label], si) => `<div class="stat" style="--s:${si}"><div class="num"><span class="rs"><span>${fmt(v)}</span></span></div><p>${label}</p></div>`).join("");
new IntersectionObserver(es => es.forEach(e => box.classList.toggle("in", e.isIntersecting)), { threshold: 0, rootMargin: "0px 0px -6% 0px" }).observe(box);
