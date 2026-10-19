import settings from "./settings";
import { Revision } from "./record";

export class Environment {
  public get isDevelopment() {
    return false;
  }
}

export function migrate(limit: number) {
  const work = async (page: number) => {
    let revisions: Revision[] = [];
    for (const revision of revisions) {
      if (revision.changed()) {
        //         ^ d: none
        return revision.stamp + page + limit;
        //               ^ d: tea/record.ts:4
      }
    }
    return settings.isDevelopment;
    //              ^ d: tea/settings.ts:2
  };
  return work;
}
