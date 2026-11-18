export class Stylus {
  blot(): string {
    return "stylus";
  }
}

class Nib {
  blot(): string {
    return "nib";
  }
}

export function a(s: Stylus): string {
  return "s.blot()" + 's.blot()';
  //        ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
  //                     ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
}

export function b(s: Stylus): string {
  return `s.blot() ${s.blot()}`;
  //        ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
  //                   ^ d: shop/quoted.ts:2
}

export function c(s: Stylus): string {
  return `${`${s.blot()}`} s.blot() \${s.blot()}`;
  //             ^ d: shop/quoted.ts:2
  //                         ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
  //                                     ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
}

export function d(s: Stylus): string {
  return `
    s.blot()
  //  ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
  `;
}

export function e(s: Stylus): string {
  return "a\\" + s.blot() + "\"" + s.blot(); // "
  //               ^ d: shop/quoted.ts:2
  //                                  ^ d: shop/quoted.ts:2
}

export function f(s: Stylus): string {
  return `${{ k: "}" }.k}` + s.blot();
  //                           ^ d: shop/quoted.ts:2
}

export function g(s: Stylus, x: string): string {
  return /'/.test(x) ? s.blot() : "";
  //                     ^ d: shop/quoted.ts:2
}

export const ink = (
  s: Stylus,
): string => s.blot();
//             ^ d: shop/quoted.ts:2

export const pen = (
  s: Stylus,
): string[] => [1].map((s: Nib) => s.blot());
//                                   ^ d: shop/quoted.ts:8

declare function retry(f: unknown, n: number): Promise<string>;
export function run(s: Stylus) {
  return retry(
    function (s: Nib) { return s.blot(); },
    3,
  ).then(() => s.blot());
  //             ^ d: shop/quoted.ts:2
}

export function wrap(s: Stylus) {
  const h = (
    f = (s: Nib) => s,
  ): string => s.blot();
  //             ^ d: shop/quoted.ts:2
  return h;
}
