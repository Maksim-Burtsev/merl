import { Flask, Urn } from "./urn";

declare const box: { u: Flask };

const u = new Urn();

export function blockOnOneLine(u: Urn) {
  { const { u } = box; u.siphon(); }
  //                     ^ d: picker tea/urn.ts:2, tea/urn.ts:8
  { let u = new Flask(); u.siphon(); }
  //                       ^ d: picker tea/urn.ts:2, tea/urn.ts:8
}

export function declaredOnTheLine() {
  const u = new Flask(); return u.siphon();
  //                              ^ d: picker tea/urn.ts:2, tea/urn.ts:8
}

export function hoistedFromABlock(on: boolean) {
  if (on) {
    var u = new Flask();
  }
  return u.siphon();
  //       ^ d: tea/urn.ts:8
}

export function hoistedFromBelow() {
  u.siphon();
  //^ d: tea/urn.ts:8
  var u = new Flask();
}

export function varOfANestedFunction() {
  u.siphon();
  //^ d: tea/urn.ts:2
  function inner() {
    var u = new Flask();
    return u;
  }
  [box].forEach((b) => { var u = b.u; return u; });
  return inner;
}

export function varInAString() {
  const s = "var u = new Flask();";
  return u.siphon() + s;
  //       ^ d: tea/urn.ts:2
}

export function controls(u: Urn) {
  if (u) { const u = new Flask(); u.siphon(); } u.siphon();
  //                                ^ d: picker tea/urn.ts:2, tea/urn.ts:8
  //                                              ^ d: tea/urn.ts:2
  u.siphon(); { const u = new Flask(); u.siphon(); }
  //^ d: tea/urn.ts:2
  //                                     ^ d: picker tea/urn.ts:2, tea/urn.ts:8
  u.siphon(); [1].forEach(() => { let u = new Flask(); return u; });
  //^ d: tea/urn.ts:2
  u.siphon(); // const u = new Flask(); u.siphon();
  //^ d: tea/urn.ts:2
  const s = "const u = 1; u.siphon()"; u.siphon();
  //                                     ^ d: tea/urn.ts:2
  const same = u == box.u; u.siphon();
  //                         ^ d: tea/urn.ts:2
  const o = { u: 1, w: u }; u.siphon();
  //                          ^ d: tea/urn.ts:2
  box.u = new Flask(); u.siphon();
  //                     ^ d: tea/urn.ts:2
  for (const n of [1]) { const u = new Flask(); u.siphon(); } u.siphon();
  //                                                            ^ d: tea/urn.ts:2
  return [s, same, o];
}
