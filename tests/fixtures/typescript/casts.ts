import { AuditLog, UserRepository } from "./repos";

export function first(found: unknown, id: number): void {
  const repo = found as UserRepository;
  void repo.deleteUser(id);
  const audit = found as unknown as AuditLog;
  audit.deleteUser(id + 1);
  void (found as UserRepository).deleteUser(id + 2);
  const loose = found as any;
  loose.deleteUser(id + 3);
  const partial = found as Partial<UserRepository>;
  void partial.deleteUser?.(id + 4);
}
