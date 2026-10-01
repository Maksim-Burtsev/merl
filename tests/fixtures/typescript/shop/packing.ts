// Names a barrel of the shop hands on from packages (#527): into the installed package, or to
// the import when the package is not installed.
import { sealCrate, stampParcel, wrapParcel } from "./kit";

export const wrapped = wrapParcel(1);
//                     ^ d: node_modules/parcel-kit/index.d.ts:1
export const sealed = sealCrate(3);
//                    ^ d: shop/packing.ts:3
//                    status: via import crate-seal (not installed)
export const stamped = stampParcel(4);
//                     ^ d: none
