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

/** @param {Urn} u */
export function hoistedBelowAParameter(u) {
  u.siphon();
  //^ d: tea/urn.ts:2
  if (!u) {
    var u = new Flask();
  }
}

export function hoistedBelowAnAssignment() {
  var u = new Urn();
  u.siphon();
  //^ d: tea/urn.ts:2
  for (const n of [1]) {
    var u = new Flask();
  }
}
