export class Crane {
  hoistCargo(load: number): number {
    return load;
  }
  swingCargo<
    TArm extends number,
  >(arm: TArm) {
    return arm;
  }
  constructor(private readonly jib: number) {}
  describeCrane(): string {
    return this.constructor.name;
    //          ^ d: shop/heads.ts:10
  }
}

export function rigCrane(): void {
  hoistCargo(1, {
  });
  hoistCargo(this, {
  });
  hoistCargo<number>(2, {
  });
  swingCargo<
    number,
  >(3);
}

export function workCrane(c) {
  c.hoistCargo(4);
  //^ d: shop/heads.ts:2
  c.swingCargo(5);
  //^ d: shop/heads.ts:5
}
