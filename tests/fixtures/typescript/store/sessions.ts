export class Session {
  static start(): Session {
    return new Session();
  }

  close(): void {}
}

export class Pool {
  static start(): Pool {
    return new Pool();
  }
}

export function openSession(): Session {
  return Session.start();
}
