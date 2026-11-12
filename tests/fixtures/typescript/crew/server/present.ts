import { presentSquad as presentSquadBase, presentCrew, presentTeam, shapes } from "./presenters";

export function presentSquad(name: string) {
  return presentSquadBase(name);
  //     ^ d: crew/server/presenters/squad.ts:1
}

export function presentMember(name: string) {
  return presentCrew(name);
  //     ^ d: crew/server/presenters/member.ts:1
}

export function presentAll(name: string) {
  presentTeam(name);
  // ^ d: crew/server/presenters/index.ts:5
  return shapes.presentShape(name);
  //            ^ d: crew/server/presenters/shapes.ts:1
}
