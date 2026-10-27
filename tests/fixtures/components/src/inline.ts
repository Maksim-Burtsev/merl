import { greeting } from "./names";
export const out = String(greeting, [] as Array<{ greeting: string }>);
//                        ^ d: src/names.ts:5
export function pick(slug: string, rows: Array<{ slug: string }>) {
//                   ^ d: none
  return rows;
}

export interface Note {
  greeting: string;
  slug: string;
}
