import { Pawl } from './pawl';
import { Winch } from './winch';

export function run(w: Winch) {
  console.log(w.slack);
  //            ^ d: winch/winch.ts:5
  w.slack = new Pawl();
  //^ d: winch/winch.ts:6
  w.slack.click();
  //^ d: winch/winch.ts:5
  //      ^ d: winch/pawl.ts:2
  if (w.slack === undefined) {
    //  ^ d: winch/winch.ts:5
    return;
  }
  console.log(w.width);
  //            ^ d: winch/winch.ts:7
}

const x = new Winch();
x.slack.click();
//^ d: winch/winch.ts:5
x.slack = new Pawl();
//^ d: winch/winch.ts:6
