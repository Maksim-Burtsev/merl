package ratelimit

import "math"

// burstSeconds is how many seconds of the plan's rate a full bucket holds, so a
// client can send a short burst instead of spreading its requests over the minute.
const burstSeconds = 6

// BurstFor returns the capacity of a bucket on the plan, never below its configured burst.
func BurstFor(plan Limits) int {
	return max(plan.Burst, int(math.Ceil(float64(plan.RatePerMinute)*burstSeconds/60)))
}
