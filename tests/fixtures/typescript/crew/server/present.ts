import { presentSquad as presentSquadBase, presentCrew } from "./presenters";

export function presentSquad(name: string) {
  return presentSquadBase(name);
  //     ^ d: crew/server/presenters/squad.ts:1
}

export function presentMember(name: string) {
  return presentCrew(name);
  //     ^ d: crew/server/presenters/member.ts:1
}
