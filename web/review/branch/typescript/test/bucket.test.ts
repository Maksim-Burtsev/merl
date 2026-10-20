import { describe, expect, it } from "vitest";

import { TokenBucket } from "../src/bucket.js";
import { Limits } from "../src/plans.js";
import { MemoryStore } from "../src/store.js";

const limits = new Limits(60, 3);

describe("TokenBucket", () => {
  it("spends the burst, then refuses", () => {
    const bucket = limits.newBucket(0);
    expect([0, 0, 0, 0].map((now) => bucket.take(now))).toEqual([true, true, true, false]);
  });

  it("refills at the rate", () => {
    const bucket = limits.newBucket(0);
    for (let i = 0; i < 3; i++) bucket.take(0);
    expect(bucket.take(1)).toBe(true);
    expect(bucket.take(1)).toBe(false);
  });

  it("asks to retry once one token is back", () => {
    const bucket = limits.newBucket(0);
    for (let i = 0; i < 3; i++) bucket.take(0);
    expect(bucket.retryAfter(0.25)).toBe(1);
  });

  it("rounds a short wait up to a second", () => {
    const bucket = limits.newBucket(0);
    for (let i = 0; i < 3; i++) bucket.take(0);
    expect(bucket.retryAfter(0.999)).toBe(1);
  });

  it("needs no wait with tokens left", () => {
    expect(limits.newBucket(0).retryAfter(0)).toBe(0);
  });

  it("sizes the burst from the plan", () => {
    const bucket = TokenBucket.forPlan(new Limits(600, 2), 0);
    const taken = Array.from({ length: 100 }, () => bucket.take(0));
    expect(taken.filter(Boolean)).toHaveLength(60);
  });
});

describe("MemoryStore", () => {
  it("round-trips a bucket", async () => {
    const store = new MemoryStore();
    const bucket = limits.newBucket(10);
    bucket.take(10);
    await store.set("key-1", bucket, 60);
    expect(await store.get("key-1")).toEqual(bucket);
  });
});
