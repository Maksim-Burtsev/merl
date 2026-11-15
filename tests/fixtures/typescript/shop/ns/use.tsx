import * as Box from "./box";
import * as Other from "./box";
import { Popover } from "./index";

export const a = <Box.Part />;
//                ^ d: shop/ns/use.tsx:1
export const b = <Other.Part />;
//                ^ d: shop/ns/use.tsx:2
export const c = Box.Part;
//               ^ d: shop/ns/use.tsx:1
//                   ^ d: shop/ns/box.tsx:2
export const p = <Popover.Title />;
//                ^ d: shop/ns/popover.tsx:1
//                        ^ d: shop/ns/popover.tsx:1
