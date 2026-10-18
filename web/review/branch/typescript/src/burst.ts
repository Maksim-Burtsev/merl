import type { Limits } from "./plans.js";

// A full bucket holds this many seconds of the plan's rate, so a client can send
// a short burst instead of spreading its requests evenly over the minute.
export const BURST_SECONDS = 6;

export function burstFor(plan: Limits): number {
  return Math.max(plan.burst, Math.ceil((plan.ratePerMinute * BURST_SECONDS) / 60));
}
