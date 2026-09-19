import { UnitOfWork } from "./chains";
import * as parts from "./nest_parts";
import { Outer } from "./nest_parts";
import { AuditLog, UserRepository } from "./repos";

const repo = new AuditLog();

function Inject(token: string): ParameterDecorator {
  return () => console.log(token);
}

// A NestJS service: the dependencies are constructor parameters, one to a line.
export class AlbumService {
  constructor(
    @Inject("repo") private repo: UserRepository,
    private readonly audit: AuditLog,
    @Inject("uow")
    protected uow: UnitOfWork,
    plain: AuditLog,
  ) {
    console.log(plain);
  }

  remove(id: number): void {
    void this.repo.deleteUser(id);
    this.audit.deleteUser(id + 1);
    void this.uow.users.deleteUser(id + 2);
    console.log(this.repo, this.audit, this.uow);
  }

  // `const { repo } = this` hands the fields on, under their names or new ones.
  sweep(id: number): void {
    const { repo, audit: trail } = this;
    void repo.deleteUser(id + 3);
    trail.deleteUser(id + 4);
    const { users } = this.uow;
    void users.deleteUser(id + 5);
    const { uow = new UnitOfWork(), ...rest } = this;
    uow.audit.deleteUser(id + 6);
    console.log(rest);
  }
}

namespace Local {
  export class Tool {
    turn(times: number): void {
      console.log(times);
    }
  }
}

export class Tool {
  turn(times: number): void {
    console.log(times);
  }
}

// A class behind namespaces, of this file's import and of a module taken whole.
export function build(id: number): void {
  const widget = new Outer.Inner.Widget();
  widget.spin(id);
  new Outer.Inner.Widget().spin(id + 1);
  const other = new parts.Outer.Inner.Widget();
  other.spin(id + 2);
  const gadget = new Outer.Gadget();
  gadget.spin(id + 3);
  repo.deleteUser(id + 7);
  const tool = new Local.Tool();
  tool.turn(id);
  new Local.Tool().turn(id + 1);
  new Tool().turn(id + 2);
}
