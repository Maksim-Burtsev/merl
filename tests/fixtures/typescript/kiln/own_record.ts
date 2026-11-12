import { Kiln } from './kiln';

class Record<K, V> {
  [key: string]: Pail;
}

class Pail {
  boil(): number {
    return 4;
  }
}

export function ownRecord(byName: Record<string, Kiln>) {
  byName['a'].boil();
  //          ^ d: !jump
}
