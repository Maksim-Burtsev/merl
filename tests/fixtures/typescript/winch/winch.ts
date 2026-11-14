import { Pawl } from './pawl';

export class Winch {
  private _slack = new Pawl();
  get slack(): Pawl { return this._slack; }
  set slack(v: Pawl) { this._slack = v; }
  get width(): number { return 1; }

  grow() {
    this.slack.click();
    //   ^ d: winch/winch.ts:5
    this.slack = new Pawl();
    //   ^ d: winch/winch.ts:6
  }
}

function Column(): PropertyDecorator {
  return () => undefined;
}

export class Hoist {
  @Column()
  get rope(): string {
    return '';
  }

  set rope(v: string) {
    console.log(v);
  }
}
