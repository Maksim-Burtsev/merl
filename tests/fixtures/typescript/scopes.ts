import { AuditLog, UserRepository } from "./repos";

const ledger = new AuditLog();

export function rotate(id: number): void {
  const ledger = new UserRepository();
  ledger.findUser(id);
}

export function each(repos: UserRepository[], id: number): void {
  repos.forEach((ledger: UserRepository) => {
    void ledger.deleteUser(id);
  });
}

export function nested(id: number): void {
  const ledger = new AuditLog();
  if (id > 0) {
    const ledger = new UserRepository();
    void ledger.deleteUser(id + 1);
  }
  ledger.deleteUser(id + 2);
}

export function hidden(ledger: any, id: number): void {
  ledger.deleteUser(id + 3);
}

export function sameLine(repos: UserRepository[], id: number): void {
  ledger.deleteUser(repos.map((ledger: UserRepository) => ledger).length + id);
}

export function continued(id: number): unknown[] {
  return [(ledger: UserRepository) => ledger,
    ledger.deleteUser(id + 4)];
}
