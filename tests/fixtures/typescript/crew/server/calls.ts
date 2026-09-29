export function enlist<T>(x: number): T {
  return x as unknown as T;
}

enlist<
  string
>(1);

export const two = enlist<number>(2);
//                 ^ d: crew/server/calls.ts:1
// status: local
