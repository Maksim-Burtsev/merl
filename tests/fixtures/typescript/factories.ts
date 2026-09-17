import { AuditLog, UserRepository } from "./repos";
import connect from "./store";

export function makeRepo(): UserRepository {
  return new UserRepository();
}

export function makeAudit() {
  return new AuditLog();
}

export async function rotate(id: number): Promise<void> {
  const repo = makeRepo();
  await repo.deleteUser(id);
  const audit = makeAudit();
  audit.deleteUser(id);
  const session = connect();
  session.close();
}

export function forget(repo: any, id: number): void {
  repo.findUser(id);
  void repo.deleteUser(id);
}
