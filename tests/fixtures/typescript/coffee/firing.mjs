import fire from "./kiln.mjs";
import pot from "./kettle.mjs";

export const fired = new fire();
//                       ^ d: coffee/kiln.mjs:1
export const boiled = pot.boil();
//                    ^ d: coffee/kettle.mjs:7
//                        ^ d: coffee/kettle.mjs:2
