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
}
