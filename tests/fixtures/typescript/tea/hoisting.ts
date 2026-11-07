import { Flask, Urn } from "./urn";

const u = new Urn();

export function outerVarBelowTheInnerFunction() {
  function inner() {
    return u.siphon();
    //       ^ d: tea/urn.ts:8
  }
  var u = new Flask();
  return inner;
}

export function outerVarInABlockAboveAnArrow(on: boolean) {
  if (on) {
    var u = new Flask();
  }
  const f = () => {
    return u.siphon();
    //       ^ d: tea/urn.ts:8
  };
  return f;
}

export function outerVarTwoFunctionsUp() {
  return function middle() {
    return () => {
      return u.siphon();
      //       ^ d: tea/urn.ts:8
    };
  };
  var u = new Flask();
}

export function innerParameterHidesTheOuterVar() {
  function inner(u: Urn) {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  var u = new Flask();
  return inner;
}

export function innerLetHidesTheOuterVar() {
  function inner() {
    let u = new Urn();
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  var u = new Flask();
  return inner;
}

export function varOfASiblingFunction() {
  function reader() {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  function writer() {
    var u = new Flask();
    return u;
  }
  return [reader, writer];
}

export class Holder {
  read() {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  write() {
    var u = new Flask();
    return u;
  }
}

export function outerVarInAString() {
  function inner() {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  const s = "var u = new Flask();";
  return [inner, s];
}

export function outerUntypedVar(x: unknown) {
  function inner() {
    return u.siphon();
    //       ^ d: picker tea/urn.ts:2, tea/urn.ts:8
  }
  var u = x;
  return inner;
}

export function arrowParameterHidesTheOuterVar() {
  const f = (u: Urn) => {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  };
  var u = new Flask();
  return f;
}

export function catchParameterHidesTheOuterVar() {
  function inner() {
    try {
      return 0;
    } catch (u) {
      return u.siphon();
      //       ^ d: !jump
    }
  }
  var u = new Flask();
  return inner;
}

export function innerDestructureHidesTheOuterVar() {
  function inner() {
    const { u } = { u: new Urn() };
    return u.siphon();
    //       ^ d: !jump
  }
  var u = new Flask();
  return inner;
}

export function varOfAOneLineMethod() {
  const o = { m() { var u = new Flask(); return u; } };
  function inner() {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  return [o, inner];
}

export function varOfAOneLineClass() {
  class K { m() { var u = new Flask(); return u; } }
  function inner() {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  return [K, inner];
}

export function varOfAnAllmanSibling() {
  function sib()
  {
    var u = new Flask();
    return u;
  }
  function inner() {
    return u.siphon();
    //       ^ d: tea/urn.ts:2
  }
  return [sib, inner];
}

export function blockConstBetweenTheVarAndAnArrow(on: boolean) {
  var u = new Flask();
  if (on) {
    const u = new Urn();
    const f = () => {
      return u.siphon();
      //       ^ d: tea/urn.ts:2
    };
    return f;
  }
  return u;
}
