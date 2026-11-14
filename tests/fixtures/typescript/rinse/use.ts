import { Cup, Mug } from "./cup";
import "./rinse";
import "./mug_rinse";

const cup: Cup = new Cup();
const mug = new Mug();
cup.rinse();
//  ^ d: rinse/rinse.ts:5
mug.rinse();
//  ^ d: rinse/mug_rinse.ts:5
