import { AuditLog } from "./repos";

export class Archive {
  label = "archive";

  constructor(protected audit: AuditLog) {}

  store(item: number): void {}

  flush(): void {}

  toString(): string {
    return this.label;
  }
}

export class ColdArchive extends Archive {
  store(item: number): void {
    super.store(item);
  }
}

export class GlacierArchive extends ColdArchive {
  store(item: number): void {
    super.store(item + 1);
    super.flush();
    [2].forEach((n) => super.store(n));
  }

  flush(): void {
    const plain = {
      toString(): string {
        return "plain " + super.toString();
      },
    };
    void plain;
  }
}

export class Vault {
  open(): Archive {
    return new Archive(new AuditLog());
  }
}

export class ColdVault extends Vault {
  open(): ColdArchive {
    const opened = super.open();
    opened.store(1);
    super.open().store(2);
    return new ColdArchive(new AuditLog());
  }
}
