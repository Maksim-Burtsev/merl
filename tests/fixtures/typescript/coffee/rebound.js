import { Flask, Urn } from "../tea/urn";

/** @type {Urn} */
const u = new Urn();

export function hoistedFromABlock(on) {
  if (on) {
    var u = new Flask();
  }
  return u.siphon();
  //       ^ d: tea/urn.ts:8
}

export function hoistedInASwitch(n) {
  switch (n) {
    case 1:
      var u = new Flask();
      break;
  }
  return u.siphon();
  //       ^ d: tea/urn.ts:8
}
