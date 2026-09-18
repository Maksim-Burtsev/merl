import { AuditLog, UserRepository } from "./repos";

const ledger = new AuditLog();

export function rotate(id: number): void {
  const ledger = new UserRepository();
  ledger.findUser(id);
}

export function each(repos: UserRepository[], id: number): void {
  repos.forEach((ledger: UserRepository) => {
    void ledger.deleteUser(id);
  });
}

export function nested(id: number): void {
  const ledger = new AuditLog();
  if (id > 0) {
    const ledger = new UserRepository();
    void ledger.deleteUser(id + 1);
  }
  ledger.deleteUser(id + 2);
}

export function hidden(ledger: any, id: number): void {
  ledger.deleteUser(id + 3);
}

export function sameLine(repos: UserRepository[], id: number): void {
  ledger.deleteUser(repos.map((ledger: UserRepository) => ledger).length + id);
}

export function continued(id: number): unknown[] {
  return [(ledger: UserRepository) => ledger,
    ledger.deleteUser(id + 4)];
}

function register(each: unknown, options: unknown): unknown[] {
  return [each, options];
}

export function literal(id: number): unknown[] {
  return register((ledger: UserRepository) => ledger, {
    done: ledger.deleteUser(id + 5),
  });
}

// What a sibling block or a callback on the header's line declares is not the cursor's.
export function sibling(ledger: AuditLog, repos: UserRepository[], id: number): void {
  try {
    repos.forEach((ledger: UserRepository) => ledger.findUser(id));
  } catch (e) {
    ledger.deleteUser(id + 6);
  }
  if (id === 1) {
    for (const ledger of repos) {
      void ledger;
    }
  } else if (id === 2) {
    ledger.deleteUser(id + 7);
  }
  if (repos.some((ledger: UserRepository) => ledger !== null)) {
    ledger.deleteUser(id + 8);
  }
  repos.filter((ledger: UserRepository) => ledger !== null).forEach((found) => {
    ledger.deleteUser(id + 9 + Number(found === null));
  });
}

// An arrow function whose line ends in `=>` opens its body below it.
export function wrapped(repos: UserRepository[], id: number): void {
  repos.forEach((ledger: UserRepository) =>
    ledger.deleteUser(id + 10),
  );
}

interface Deps {
  ledger: UserRepository;
  id: number;
}

// A destructured parameter wrapped by prettier: `}: Deps): void {` closes the parameters, not a
// sibling block, and its `ledger` has no type the rules read.
export function destructured({
  ledger,
  id,
}: Deps): void {
  void ledger.deleteUser(id + 11);
}

// A backtick in a regex opens no template.
export function ticked(id: number): string {
  const parts = "a/b".split(/`/);
  const ledger = new UserRepository();
  void ledger.deleteUser(id + 12);
  return parts.join("`");
}
