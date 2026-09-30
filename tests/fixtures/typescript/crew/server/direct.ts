import Member from "./models/Member";
import type { ApiContext } from "./types";

export async function handler(ctx: ApiContext) {
//                                 ^ d: crew/server/types.ts:1
  return Member.build();
  //     ^ d: crew/server/models/Member.ts:2
  // status: via import crew/server/models/Member.ts
}
