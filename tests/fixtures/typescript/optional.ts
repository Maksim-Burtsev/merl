import { UnitOfWork } from "./chains";
import { AuditLog, UserRepository } from "./repos";

const note = new AuditLog();

// `r!.m()` and `a?.b.m()` have the type of the plain access for a member lookup.
export class Clerk {
  private repo?: UserRepository;

  constructor(private uow?: UnitOfWork) {}

  clear(id: number, spare?: AuditLog, found?: any, u?: UnitOfWork): void {
    void this.repo!.deleteUser(id);
    void this.repo?.deleteUser(id + 1);
    void this.uow?.users.deleteUser(id + 2);
    this.uow!.audit!.deleteUser(id + 3);
    spare?.deleteUser(id + 4);
    spare!.deleteUser(id + 5);
    found?.deleteUser(id + 6);
    u!.users!.deleteUser(id + 9);
    // A negation is no mark.
    console.log(!note.deleteUser.length);
  }
}
