import { AuditLog, Notifier, UserRepository } from "./repos";

export class UserService {
  private audit = new AuditLog();

  constructor(
    private repo: UserRepository,
    private notifier: Notifier,
  ) {}

  async remove(id: number): Promise<void> {
    const user = this.repo.findUser(id);
    await this.repo.deleteUser(id);
    this.audit.deleteUser(id);
    this.notifier.send(`removed ${user.id}`);
  }
}

export class Services {
  constructor(public users: UserService) {}
}

export class App {
  constructor(public services: Services) {}
}

export async function handleDelete(app: App, id: number): Promise<void> {
  await app.services.users.remove(id);
}

export function cleanup(id: number): void {
  const repo = new AuditLog();
  const purge = async () => {
    const repo = new UserRepository();
    await repo.deleteUser(id);
  };
  void purge;
  repo.deleteUser(id);
}
