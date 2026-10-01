export interface User {
  name: string;
}

export function loadUser(): User {
  return { name: "Ada" };
}

export class Team {
  name = "core";
}
