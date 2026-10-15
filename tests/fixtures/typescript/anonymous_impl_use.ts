import Whirl from "./anonymous_impl";

export function whirl(): void {
  const w = new Whirl();
  w.spin();
  //^ d: anonymous_impl.ts:2
  //  status: spin → Whirl.spin (via w: Whirl)
}
