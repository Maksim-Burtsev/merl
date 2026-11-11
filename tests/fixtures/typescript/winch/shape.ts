import { Hoist, Winch } from './winch';

export function shape(w: Winch) {
  return {
    slack: w.slack,
    //       ^ d: winch/winch.ts:5
  };
}

export function hoisted(h: Hoist) {
  h.rope = 'x';
  //^ d: winch/winch.ts:27
  h.rope += 'x';
  //^ d: picker winch/winch.ts:23, winch/winch.ts:27
  const { rope } = h;
  return {
    rope: h.rope,
    //      ^ d: winch/winch.ts:23
    other: [h.rope, rope],
    //        ^ d: picker winch/winch.ts:23, winch/winch.ts:27
  };
}
