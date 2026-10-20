import type { Redis } from "ioredis";

import { TokenBucket, type BucketFields } from "./bucket.js";

// Where buckets live between requests. A get and the set after it are not atomic,
// so two requests at once may both spend the same token.
export interface BucketStore {
  get(key: string): Promise<TokenBucket | undefined>;
  set(key: string, bucket: TokenBucket, ttlSeconds: number): Promise<void>;
}

// Keeps buckets in a Map with Redis-like expiry. For tests and local runs.
export class MemoryStore implements BucketStore {
  private readonly items = new Map<string, { fields: BucketFields; expiresAt: number }>();

  async get(key: string): Promise<TokenBucket | undefined> {
    const item = this.items.get(key);
    if (!item) {
      return undefined;
    }
    if (item.expiresAt <= performance.now()) {
      this.items.delete(key);
      return undefined;
    }
    return TokenBucket.fromFields(item.fields);
  }

  async set(key: string, bucket: TokenBucket, ttlSeconds: number): Promise<void> {
    this.items.set(key, {
      fields: bucket.toFields(),
      expiresAt: performance.now() + ttlSeconds * 1000,
    });
  }
}

export class RedisStore implements BucketStore {
  constructor(
    private readonly redis: Redis,
    private readonly prefix = "ratelimit:",
  ) {}

  async get(key: string): Promise<TokenBucket | undefined> {
    const fields = await this.redis.hgetall(this.prefix + key);
    if (Object.keys(fields).length === 0) {
      return undefined;
    }
    return TokenBucket.fromFields(fields);
  }

  async set(key: string, bucket: TokenBucket, ttlSeconds: number): Promise<void> {
    const name = this.prefix + key;
    await this.redis.multi().hset(name, bucket.toFields()).expire(name, ttlSeconds).exec();
  }
}
