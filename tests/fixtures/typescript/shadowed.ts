import { AuditLog, UserRepository } from "./repos";

// A namespace of the file, and a parameter of its name that is something else.
namespace svc {
  export function list(): AuditLog[] {
    return [];
  }

  export class Tool {
    turn(times: number): void {
      console.log(times);
    }
  }
}

interface Remote {
  list(): UserRepository[];
  Tool: new () => { turn(times: string): void };
}

export function own(id: number): void {
  const logs = svc.list();
  for (const log of logs) {
    log.deleteUser(id);
  }
}

export function sweep(svc: Remote, id: number): void {
  const repos = svc.list();
  for (const r of repos) {
    void r.deleteUser(id + 1);
  }
  const tool = new svc.Tool();
  tool.turn(String(id + 2));
}
