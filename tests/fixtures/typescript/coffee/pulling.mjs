import { Machine, makeLever, parts } from "./espresso.mjs";
import { padStart as pad } from "./lever.mjs";

export function pullAll(names) {
  const label = names.join(" ").padStart(4);
  //                            ^ d: coffee/lever.mjs:32; want none (#1145)
  const cups = label.split(",").filter(Boolean);
  //                            ^ d: coffee/lever.mjs:34; want none (#1145)
  const machine = new Machine();
  //                  ^ d: coffee/lever.mjs:17
  const time = machine
    .lock()
   //^ d: coffee/lever.mjs:8
    .release()
   //^ d: coffee/lever.mjs:26
    .shower()
   //^ d: coffee/lever.mjs:12
    .filter(cups.length);
   //^ d: coffee/lever.mjs:34
  return time + makeLever().shower().padStart + machine.flush() + machine.pressure;
  //                        ^ d: coffee/lever.mjs:12
  //                                 ^ d: coffee/lever.mjs:32
  //                                                    ^ d: coffee/lever.mjs:18
  //                                                                      ^ d: coffee/lever.mjs:2
}

export function eachShot(shot, shots) {
  let total = 0;
  for (const shot of shots) {
    total += shot;
    //       ^ d: coffee/pulling.mjs:29
  }
  return total + shot;
  //             ^ d: coffee/pulling.mjs:27
}

export function weigh(names) {
  return names.map((pad) => pad.length + inner());
  //                        ^ d: coffee/pulling.mjs:38
  //                                     ^ d: picker coffee/pulling.mjs:42, …; want coffee/pulling.mjs:42 (#1146)

  function inner() {
    return pad("x").length;
    //     ^ d: coffee/lever.mjs:47
  }
}

export function unpack(lever) {
  const { pressure, flush } = lever;
  return pressure + flush.call(lever);
  //     ^ d: coffee/pulling.mjs:49
  //                ^ d: coffee/pulling.mjs:49
}

export function viaParts() {
  return new parts.Shower().filter(1);
  //               ^ d: picker coffee/lever.mjs:31; want coffee/lever.mjs:31 (#1147)
  //                        ^ d: coffee/lever.mjs:34
}

export async function late() {
  const lazy = await import("./lever.mjs");
  return lazy.makeLever().pull(2);
  //          ^ d: coffee/lever.mjs:43
  //                      ^ d: coffee/lever.mjs:22
}

export function settled() {
  return Promise.resolve(1).then((n) => n);
  //                        ^ d: picker coffee/lever.mjs:38, tea/strainer.ts:41; want none (#1145)
}

/** @param {import("./lever.mjs").Shower} shower */
export function rinse(shower) {
  return shower.filter(1);
  //            ^ d: coffee/lever.mjs:34
}

export const recipe = `
export class Lever {
  pull() { return ${makeLever().pull(1)}; }
  //                            ^ d: coffee/lever.mjs:22
}
`;

// export function makeLever() {}
export function brewShot() {
  const Cup = class Demitasse {
    sip() {
      return this.size();
      //          ^ d: picker coffee/pulling.mjs:94, …; want coffee/pulling.mjs:94 (#1148)
    }

    size() {
      return 1;
    }
  };
  return new Cup().sip() + makeLever().release().pull(1);
  //                       ^ d: coffee/lever.mjs:43
  //                                             ^ d: coffee/lever.mjs:22
}
