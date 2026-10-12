import Spinner from "./anonymous";

export function twirl(): void {
  const s = new Spinner();
  s.spin();
  //^ d: picker anonymous.ts:2, nest_parts.ts:4, nest_parts.ts:11, nest_parts.ts:18; want anonymous.ts:2 (#817)
}
