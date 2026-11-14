import { Kiln } from './kiln';

class WeakMap<K, V> {
  get(k: K): Pail {
    return new Pail();
  }
}

class Pail {
  boil(): number {
    return 4;
  }
}

export function ownWeakMap(byName: WeakMap<string, Kiln>) {
  byName.get('a').boil();
  //              ^ d: kiln/own_weak_map.ts:10
}
