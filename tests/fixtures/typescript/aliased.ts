import { AuditLog } from "./repos";

// Declared under one name and exported under another, as hono exports its `HonoBase`.
class Trunk {
  audit = new AuditLog();

  lock(): void {}
}

// Another `Trunk`, inside a namespace: not the one the module exports.
namespace Spare {
  export class Trunk {
    lock(): void {}
  }
}

class Hatch {
  lock(): void {}
}

// A namesake of what the last line hands on from `repos`, which is what `HatchBase` is.
class UserRepository {
  wipe(): void {
    console.log(UserRepository);
  }
}

export {
  Hatch,
  Spare,
  Trunk as TrunkBase,
};
export { UserRepository as HatchBase } from "./repos";
