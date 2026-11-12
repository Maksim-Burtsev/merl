export class Platter {
  dollop(): void {}
}
export class Saucepan {
  dollop(): void {}
}

export function servePair(): [Platter, Saucepan] {
  return [new Platter(), new Saucepan()];
}
export function serveRest(): [Platter, ...Saucepan[]] {
  return [new Platter()];
}

export function serveMaybe(): [Platter, Saucepan] | undefined {
  return undefined;
}

export function unpack(): void {
  const [plate, pan] = servePair();
  pan.dollop();
  //  ^ d: shop/servings.ts:5
  plate.dollop();
  //    ^ d: shop/servings.ts:2
  const [, other] = servePair();
  other.dollop();
  //    ^ d: shop/servings.ts:5
  const [first, second]: [Saucepan, Platter] = [new Saucepan(), new Platter()];
  second.dollop();
  //     ^ d: shop/servings.ts:2
  first.dollop();
  //    ^ d: shop/servings.ts:5
  const [head, tail] = serveRest();
  tail.dollop();
  //   ^ d: !jump
  const [one, two, three] = servePair();
  two.dollop();
  //  ^ d: !jump
  const [maybe, perhaps] = serveMaybe();
  perhaps.dollop();
  //      ^ d: !jump
  let [h, k] = servePair();
  [h, k] = [k, h];
  h.dollop();
  //^ d: !jump
  let crowd: Platter[] = [];
  crowd = [new Platter()];
  for (const each of crowd) {
    each.dollop();
    //   ^ d: shop/servings.ts:2
  }
}

export interface Set1 { plate: Platter; readonly pan?: Saucepan }
export type Set2 = { pan: Saucepan, plate: Platter };
export function setOne(): Set1 {
  return { plate: new Platter() };
}
export function setTwo(): Set2 {
  return { pan: new Saucepan(), plate: new Platter() };
}

export function unpackSets({ plate, pan }: Set1): void {
  pan.dollop();
  //  ^ d: shop/servings.ts:5
  plate.dollop();
  //    ^ d: shop/servings.ts:2
  const { pan: p2, plate: q2 } = setTwo();
  p2.dollop();
  // ^ d: shop/servings.ts:5
  q2.dollop();
  // ^ d: shop/servings.ts:2
  const one = setOne();
  one.pan.dollop();
  //      ^ d: shop/servings.ts:5
  let { plate: swapped, pan: other2 } = setTwo();
  [swapped, other2] = [other2, swapped];
  swapped.dollop();
  //      ^ d: !jump
}
