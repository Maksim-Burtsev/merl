import { burstFor } from "./burst.js";
import type { Limits } from "./plans.js";

export type BucketFields = Record<string, string>;

// Token bucket for one API key. Times are Unix seconds.
export class TokenBucket {
  constructor(
    readonly capacity: number,
    readonly refillPerSecond: number,
    private tokens: number,
    private updatedAt: number,
  ) {}

  static forPlan(plan: Limits, now: number): TokenBucket {
    const burst = burstFor(plan);
    return new TokenBucket(burst, plan.ratePerMinute / 60, burst, now);
  }

  take(now: number): boolean {
    this.refill(now);
    if (this.tokens < 1) {
      return false;
    }
    this.tokens -= 1;
    return true;
  }

  retryAfter(now: number): number {
    this.refill(now);
    const missing = 1 - this.tokens;
    if (missing <= 0) {
      return 0;
    }
    return Math.ceil(missing / this.refillPerSecond);
  }

  private refill(now: number): void {
    const elapsed = Math.max(0, now - this.updatedAt);
    this.tokens = Math.min(this.capacity, this.tokens + elapsed * this.refillPerSecond);
    this.updatedAt = now;
  }

  toFields(): BucketFields {
    return {
      capacity: String(this.capacity),
      refillPerSecond: String(this.refillPerSecond),
      tokens: String(this.tokens),
      updatedAt: String(this.updatedAt),
    };
  }

  static fromFields(fields: BucketFields): TokenBucket {
    return new TokenBucket(
      Number(fields.capacity),
      Number(fields.refillPerSecond),
      Number(fields.tokens),
      Number(fields.updatedAt),
    );
  }
}
