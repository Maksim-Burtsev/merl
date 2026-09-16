export interface User {
  id: number;
}

export class UserRepository {
  findUser(id: number): User {
    return { id };
  }

  async deleteUser(id: number): Promise<void> {
    console.log(id);
  }
}

export class AuditLog {
  deleteUser(id: number): void {
    console.log(id);
  }
}

export interface Notifier {
  send(text: string): void;
}

export class EmailNotifier implements Notifier {
  send(text: string): void {
    console.log(text);
  }
}

export class SmsNotifier implements Notifier {
  send(text: string): void {
    console.log(text);
  }
}
