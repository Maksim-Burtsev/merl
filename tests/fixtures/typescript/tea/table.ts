import Heater, { steep } from "./kettle";
import { Pot, Boiler, Leaf, Blend, Caddy } from "./index";
import type { Vessel } from "./index";

type Recipe = { leaves: number; water: number };

/**
 * class Teapot {
 *   pour(cups: number): number
 * }
 * Serves through {@link Pot}.
 */
export function setTable(pot: Pot | undefined, recipe: Blend): number {
  //                          ^ d: tea/teapot.ts:10
  //                                                   ^ d: picker tea/vessel.ts:9, tea/vessel.ts:10
  const kettle = new Heater();
  //                 ^ d: tea/kettle.ts:1
  const boiler = new Boiler();
  //                 ^ d: tea/kettle.ts:1
  const hot = kettle.boil(95) + boiler.boil();
  //                 ^ d: tea/kettle.ts:5
  //                                   ^ d: tea/kettle.ts:4
  const poured = pot?.pour(2) ?? 0;
  //             ^ d: tea/table.ts:13
  //                  ^ d: tea/teapot.ts:15
  const lifted = pot?.lid?.lift() ? 1 : 0;
  //                  ^ d: tea/teapot.ts:13
  //                       ^ d: tea/teapot.ts:5
  return hot + poured + lifted + recipe.leaves + Blend.mix(1).water;
  //                                    ^ d: tea/vessel.ts:9
  //                                             ^ d: picker tea/vessel.ts:9, tea/vessel.ts:10
  //                                                   ^ d: tea/vessel.ts:11
  //                                                          ^ d: tea/vessel.ts:9
}

export function brew({ leaves, water: ml }: Recipe, minutes = steep.minutes): number {
  //                                        ^ d: tea/table.ts:5
  //                                                                ^ d: tea/kettle.ts:16
  return leaves + ml + steep(minutes);
  //     ^ d: tea/table.ts:36
  //              ^ d: tea/table.ts:36
  //                   ^ d: tea/kettle.ts:11
  //                         ^ d: tea/table.ts:36
}

export function strongest<T extends Vessel>(vessels: T[]): number {
  //                                ^ d: picker tea/vessel.ts:1, tea/vessel.ts:5
  //                                                 ^ d: tea/table.ts:46
  return vessels[0].pour(vessels[0].volume);
  //                ^ d: tea/vessel.ts:6
  //                                ^ d: tea/vessel.ts:2
}

export function pick(): number {
  const leaf: Leaf = Leaf.parse("black");
  //          ^ d: picker tea/vessel.ts:16, tea/vessel.ts:21
  //                      ^ d: tea/vessel.ts:22
  return leaf === Leaf.Black ? Caddy.fill() + Caddy.size : 0;
  //                   ^ d: tea/vessel.ts:18
  //                                 ^ d: tea/vessel.ts:32
  //                                                ^ d: tea/vessel.ts:28
  //                           ^ d: picker tea/vessel.ts:27, tea/vessel.ts:31
}

export const menu = `
class Kettle {
  boil(): number { return 0; }
}
function steep(minutes) {}
const Pot = 1;
`;

export function serveMenu(): number {
  return steep(menu.length) + new Pot().pour(1);
  //     ^ d: tea/kettle.ts:11
  //                              ^ d: tea/teapot.ts:10
  //                                    ^ d: tea/teapot.ts:15
}

const water = 500;

export function drain({ water }: Recipe): number {
  return water;
  //     ^ d: tea/table.ts:82
}

export function refill(vessel: Vessel): number {
  return water + vessel.pour(vessel.volume);
  //     ^ d: tea/table.ts:80
  //                    ^ d: tea/vessel.ts:6
  //                                ^ d: tea/vessel.ts:2
}

export namespace Tray {
  export const water = 2;

  export function weigh(): number {
    return water;
    //     ^ d: tea/table.ts:95
  }
}

export const notes = `
export namespace Tray {
  export const water = 3;
}
function drain() {}
`;

export function wipe(): number {
  return drain({ leaves: 1, water: 2 }) + Tray.water + Tray.weigh();
  //     ^ d: tea/table.ts:82
  //                                           ^ d: tea/table.ts:95
  //                                                        ^ d: tea/table.ts:97
}

export function emptiest<P extends Pot>(pots: P[]): number {
  return pots[0].pour(0) + pots.length;
  //             ^ d: tea/teapot.ts:15
}

export function lidOf(pot: Pot): boolean {
  const { lid } = pot;
  return lid?.lift() ?? false;
  //     ^ d: tea/table.ts:123
  //          ^ d: tea/teapot.ts:5
}
