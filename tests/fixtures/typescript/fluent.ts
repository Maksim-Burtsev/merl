import { Depot, openDepot } from "./calls";
import { UnitOfWork } from "./chains";
import { AuditLog } from "./repos";

const note = new AuditLog();

// prettier breaks a long member access in front of its dots.
export class Desk {
  constructor(
    private uow: UnitOfWork,
    private depot: Depot,
  ) {}

  async clear(id: number, found: any): Promise<void> {
    await this.uow.users
      .deleteUser(id);
    this.uow
      // The audit trail of this unit.
      .audit
      .deleteUser(id + 1);
    void this.depot
      .peopleRepo()
      .deleteUser(id + 2);
    void openDepot()
      .people.deleteUser(id + 3);
    // The line above ends in a comment, and a name in it is no receiver.
    found // note
      .deleteUser(id + 4);
    found
      .first()
      .second()
      .deleteUser(id + 5);
    found.open(
      id,
    )
      .deleteUser(id + 6);

    void [note]
      .length;
  }
}
