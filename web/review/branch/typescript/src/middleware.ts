import type { MiddlewareHandler } from "hono";

import { TokenBucket } from "./bucket.js";
import type { LimitsConfig } from "./plans.js";
import type { BucketStore } from "./store.js";

// Spends one token per request from the bucket of the caller's API key
// and answers 429 once the bucket is empty.
export function rateLimit(store: BucketStore, config: LimitsConfig): MiddlewareHandler {
  return async (c, next) => {
    const apiKey = c.req.header("x-api-key");
    if (!apiKey) {
      return c.json({ error: "missing API key" }, 401);
    }

    const limits = config.forKey(apiKey);
    const now = Date.now() / 1000;
    const bucket = (await store.get(apiKey)) ?? TokenBucket.forPlan(limits, now);
    const allowed = bucket.take(now);
    await store.set(apiKey, bucket, limits.idleTtl());

    if (!allowed) {
      const wait = bucket.retryAfter(now);
      c.header("Retry-After", String(Math.ceil(wait)));
      return c.json({ error: "rate limit exceeded" }, 429);
    }
    await next();
  };
}
