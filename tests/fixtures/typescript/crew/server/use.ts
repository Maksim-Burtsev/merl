import { Squad, squadName, nameOfSquad } from "./models";

const squad = Squad.build();
//            ^ d: crew/server/models/Squad.ts:1
// status: via import crew/server/models/Squad.ts
export const names = squadName(squad) + nameOfSquad(squad);
//                   ^ d: crew/server/models/helpers.ts:1
//                                      ^ d: crew/server/models/helpers.ts:1
