import math
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from plans import Limits

# A full bucket holds this many seconds of the plan's rate, so a client can send
# a short burst instead of spreading its requests evenly over the minute.
BURST_SECONDS = 6


def burst_for(plan: "Limits") -> int:
    return max(plan.burst, math.ceil(plan.rate_per_minute * BURST_SECONDS / 60))
