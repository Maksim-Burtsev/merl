import { AuditLog, UserRepository } from "./repos";

export interface Source {
  source(): UserRepository;
}

export class Depot {
  people = new UserRepository();

  peopleRepo(): UserRepository {
    return this.people;
  }

  trail() {
    return new AuditLog();
  }

  pick(id: number): UserRepository;
  pick(id: string): AuditLog;
  pick(id: number | string): UserRepository | AuditLog {
    return typeof id === "number" ? this.people : new AuditLog();
  }
}

export function openDepot(): Depot {
  return new Depot();
}

export function first(depot: Depot, src: Source, id: number): void {
  const repo = depot.peopleRepo();
  void repo.deleteUser(id);
  const trail = depot.trail();
  trail.deleteUser(id + 1);
  void openDepot().people.deleteUser(id + 2);
  void new Depot().people.deleteUser(id + 3);
  const sourced = src.source();
  void sourced.deleteUser(id + 4);
  void openDepot().peopleRepo().deleteUser(id + 5);
  const picked = depot.pick("a");
  picked.deleteUser(id + 7);
}

export function second(depot: any, id: number): void {
  const repo = depot.peopleRepo();
  repo.deleteUser(id + 6);
}

export function pickInline(flag: boolean) {
  if (flag) return new AuditLog();
  return new UserRepository();
}

export function third(id: number): void {
  const inline = pickInline(true);
  void inline.deleteUser(id + 8);
}
