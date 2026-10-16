import Spinner from "./anonymous";

export function twirl(): void {
  const s = new Spinner();
  s.spin();
  //^ d: anonymous.ts:2
  //  status: spin → Spinner.spin (via s: Spinner)
}
