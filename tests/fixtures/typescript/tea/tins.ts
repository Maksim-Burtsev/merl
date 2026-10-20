import type { ReadonlySet } from "immutable";

export function tins(jars: ReadonlySet): number {
  return jars.values();
  //          ^ d: none
  //          status: immutable not installed
}
