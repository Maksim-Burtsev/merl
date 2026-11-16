import { createJiti, Jiti } from "jiti";

export function load(path: string) {
  const jitiLoader = createJiti("");
  return jitiLoader.import(path);
  //                ^ d: none
  // status: via jitiLoader = createJiti(), jiti not installed
}

export function make(path: string) {
  const j = new Jiti("");
  return j.import(path);
  //       ^ d: none
  // status: via j = new Jiti(), jiti not installed
}
