import { AuditLog, UserRepository } from "./repos";

export function loadRepos(): UserRepository[] {
  return [new UserRepository()];
}

export function sweep(
  repos: UserRepository[],
  logs: ReadonlyArray<AuditLog>,
  id: number,
): void {
  for (const repo of repos) {
    void repo.deleteUser(id);
  }
  for (const log of logs) {
    log.deleteUser(id + 1);
  }
  const loaded = loadRepos();
  for (const repo of loaded) {
    void repo.deleteUser(id + 2);
  }
  // @ts-expect-error the array itself has no such member
  repos.deleteUser(id + 3);
}

export function sweepPairs(
  byName: Map<string, UserRepository>,
  repos: UserRepository[],
  id: number,
): void {
  for (const entry of byName) {
    // @ts-expect-error a Map hands out pairs
    entry.deleteUser(id + 4);
  }
  for (const index in repos) {
    // @ts-expect-error `in` hands out keys
    index.deleteUser(id + 5);
  }
}
