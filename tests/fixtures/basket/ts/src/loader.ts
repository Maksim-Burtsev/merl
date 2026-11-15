import { createJiti } from "jiti";

export function load(path: string) {
  const jitiLoader = createJiti("");
  return jitiLoader.import(path);
  //                ^ d: none
}
