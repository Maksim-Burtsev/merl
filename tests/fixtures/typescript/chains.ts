import { AuditLog, UserRepository } from "./repos";

export class UnitOfWork {
  users = new UserRepository();
  readonly audit: AuditLog = new AuditLog();
}

export class Box<T> {
  constructor(public item: T) {}
}

export class Folder {
  constructor(public parent: Folder) {}

  root(): Folder {
    return this;
  }
}

export function makeUow(): UnitOfWork {
  return new UnitOfWork();
}

export class Handler {
  constructor(
    private uow: UnitOfWork,
    private box: Box<UserRepository>,
  ) {}

  async deleteAccount(id: number, folder: Folder): Promise<void> {
    await this.uow.users.deleteUser(id);
    this.uow.audit.deleteUser(id);
    await this.box.item.deleteUser(id);
    const users = new AuditLog();
    await makeUow().users.deleteUser(id);
    users.deleteUser(id);
    folder.parent.parent.parent.parent.parent.root();
    folder.parent.parent.parent.parent.parent.parent.root();
  }
}

export class BaseRegistry {
  get audit(): AuditLog {
    return new AuditLog();
  }
}

export class Registry extends BaseRegistry {
  get users(): UserRepository {
    return new UserRepository();
  }

  override get audit() {
    return new AuditLog();
  }
}

export class Admin {
  constructor(private registry: Registry) {}

  purge(id: number): void {
    this.registry.users.deleteUser(id);
    this.registry.audit.deleteUser(id);
  }
}
