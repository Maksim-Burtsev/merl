import { Model } from "sequelize-typescript";

class Sapling extends Model {}

export class Grown extends Sapling {}

Object.assign(Grown.prototype, {
  mixed() {
    return 1;
  },
});

export function tend(g: Grown) {
  return g.mixed();
  //       ^ d: tea/grown.ts:8
}

class Pruned {}

Object.assign(Pruned.prototype, {
  mixed() {
    return 2;
  },
});
