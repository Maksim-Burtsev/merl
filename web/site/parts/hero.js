const sleep = ms => new Promise(r => setTimeout(r, ms));
const h = document.querySelector(".hero h1"), nav = h.querySelector(".h-nav");
const WORD = nav.textContent;

async function strike() {
  nav.innerHTML = [...WORD].map(c => `<span class="ch hide">${c}</span>`).join("");
  const cs = [...nav.children];
  h.classList.add("h-pre");
  await sleep(700);
  h.classList.replace("h-pre", "h-run");
  await sleep(550);
  cs[0].classList.add("cur", "first");
  await sleep(380);
  for (let i = 0; i < cs.length; i++) {
    cs[i].classList.remove("hide");
    cs.forEach((c, j) => c.classList.toggle("cur", j === i));
    cs[0].classList.remove("first");
    await sleep(70 + Math.random() * 80);
  }
  nav.classList.add("done");
  await sleep(1150);
  nav.classList.add("rest");
  await sleep(600);
  nav.textContent = WORD;
}
strike();
