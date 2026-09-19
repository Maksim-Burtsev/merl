import { AuditLog, UserRepository } from "./repos";

// `#name` is a name of its own: `#addRoute` is no `addRoute`.
export class Router {
  #repo = new UserRepository();
  #audit: AuditLog;

  constructor(audit: AuditLog) {
    this.#audit = audit;
  }

  #addRoute(path: string): void {
    console.log(path);
  }

  addRoute(path: string): void {
    this.#addRoute(path);
  }

  add(path: string, id: number, found: any): void {
    this.#addRoute(path + "/");
    void this.#repo.deleteUser(id);
    this.#audit.deleteUser(id + 1);
    console.log(this.#audit);
    found.addRoute(path);
    console.log("see route#addRoute");
  }

  same(other: Router, id: number): void {
    void other.#repo.deleteUser(id + 2);
  }
}

export class SubRouter extends Router {
  #addRoute(path: string, id: number): void {
    console.log(path, id);
  }

  mount(path: string): void {
    this.#addRoute(path, 1);
    this.addRoute(path);
  }
}
