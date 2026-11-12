import presentSquad from "./squad";
import presentMember from "./member";
import * as shapes from "./shapes";

function presentTeam(name: string) {
  return { name };
}

export { presentSquad, presentMember as presentCrew, presentTeam, shapes };
