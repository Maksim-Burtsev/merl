use crate::Limits;

/// How many seconds of the plan's rate a full bucket holds, so a client can send
/// a short burst instead of spreading its requests evenly over the minute.
const BURST_SECONDS: u32 = 6;

/// The capacity of a bucket on the plan, never below its configured burst.
pub fn burst_for(plan: &Limits) -> u32 {
    (plan.rate_per_minute * BURST_SECONDS).div_ceil(60).max(plan.burst)
}
