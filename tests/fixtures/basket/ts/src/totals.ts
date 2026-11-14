export function doubled(prices: number[]): number[] {
  const out = [];
  const typed: number[] = [];
  for (const p of prices) {
    out.push(p * 2);
    //  ^ d: none
    typed.push(p);
    //    ^ d: none
  }
  const text = "a,b";
  return text.includes("a") ? out : typed;
  //          ^ d: none
}
