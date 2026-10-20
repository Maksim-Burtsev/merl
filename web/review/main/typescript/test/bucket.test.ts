import { describe, expect, it } from "vitest";

import { Limits } from "../src/limits.js";
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
    expect(bucket.retryAfter(0.25)).toBeCloseTo(0.75);
  });

  it("needs no wait with tokens left", () => {
    expect(limits.newBucket(0).retryAfter(0)).toBe(0);
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
