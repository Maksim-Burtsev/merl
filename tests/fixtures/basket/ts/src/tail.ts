export function tail(): number {
  const xs = [1, 2];
  return xs.last();
  //        ^ d: ts/src/global.d.ts:5
}
