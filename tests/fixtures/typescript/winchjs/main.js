import { Pawl } from './pawl.js';
import { Winch } from './winch.js';

/** @param {Winch} w */
export function run(w) {
  console.log(w.slack);
  //            ^ d: winchjs/winch.js:6
  w.slack = new Pawl();
  //^ d: winchjs/winch.js:7
  w.slack.click();
  //      ^ d: winchjs/pawl.js:2
  w.spare.click();
  //      ^ d: winchjs/pawl.js:2
}

const x = new Winch();
x.slack = new Pawl();
//^ d: winchjs/winch.js:7
x.slack.click();
//      ^ d: winchjs/pawl.js:2
