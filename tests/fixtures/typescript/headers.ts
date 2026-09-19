import { AuditLog, UserRepository } from "./repos";

const repo = new AuditLog();

export interface Sealable {
  seal(key: string): void;
}

export class Crate<K> {
  protected audit = new AuditLog();

  seal(key: K): void {
    console.log(key);
  }
}

// prettier wraps a long list of type parameters: the header ends in `> extends … {`.
export class Shelf<
  K extends string = string,
  V extends object = object,
> extends Crate<K> {
  private repo = new UserRepository();

  constructor(private readonly spare: V) {
    super();
  }

  drop(id: number, key: K): void {
    void this.repo.deleteUser(id);
    this.audit.deleteUser(id + 1);
    this.seal(key);
    super.seal(key);
    console.log(this.spare);
  }
}

// The clauses on lines of their own, the `{` of the body alone.
export class LongNamedShelfOfStrings
  extends Crate<string>
  implements Sealable
{
  private repo = new UserRepository();

  drop(id: number): void {
    void this.repo.deleteUser(id + 2);
    this.audit.deleteUser(id + 3);
  }

  seal(key: string): void {
    super.seal(key + "!");
  }
}

// `S extends Sealable` is a constraint: a `Bin` seals nothing for anybody.
export class Bin<
  S extends Sealable,
  T extends Crate<string> = Crate<string>,
> {
  constructor(
    private one: S,
    private other: T,
  ) {}

  seal(key: string): void {
    this.one.seal(key);
    this.other.seal(key);
  }
}

// A wrapped list of type parameters in front of a function's parameters.
export function stock<
  K extends string,
  V extends object,
>(repo: UserRepository, key: K, spare: V): void {
  void repo.deleteUser(key.length);
  console.log(spare);
}

// A block of its own under a statement: the `{` is no body of the line above it.
export function bare(id: number): void {
  const repo = new UserRepository();
  {
    void repo.deleteUser(id + 4);
  }
}

export function audit(id: number): void {
  repo.deleteUser(id);
}
