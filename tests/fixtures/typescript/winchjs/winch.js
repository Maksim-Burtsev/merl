import { Pawl } from './pawl.js';

export class Winch {
  #slack = new Pawl();
  /** @returns {Pawl} */
  get slack() { return this.#slack; }
  set slack(v) { this.#slack = v; }
  /** @type {Pawl} */
  get spare() { return this.#slack; }

  grow() {
    this.slack.click();
    //   ^ d: winchjs/winch.js:6
    //         ^ d: winchjs/pawl.js:2
    this.slack = new Pawl();
    //   ^ d: winchjs/winch.js:7
  }
}
