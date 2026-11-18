// The name of a function expression is at a declaration, as a function's is: namesakes are offered (#1536).
export function customLoader(ext: string) {
  return async function loader(path: string) {
//                      ^ d: picker shop/loader_keys.ts:2
//                      status: loader: at a declaration, 1 other by name
    return path + ext;
  };
}
export const mapped = [1].map(function* mapper(n) { yield n; });
//                                      ^ d: picker shop/loader_keys.ts:3
export const lone = [1].map(function lonelyMapper(n) { return n; });
//                                   ^ d: none
