import { type TreeNode } from "./types";

export const node = (
  id: string
): TreeNode => ({
// ^ d: crew/server/types.ts:7
  id,
});

export const leaf = (id: string): TreeNode => ({ id });
//                                ^ d: crew/server/types.ts:7
// status: TreeNode: via import crew/server/types.ts

// A key's `:` in front of an arrow's bare parameter is no return type.
export const handlers = {
  press: e =>
    0 + e.length,
    //  ^ d: crew/server/tree.ts:16
};
