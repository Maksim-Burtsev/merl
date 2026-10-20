import { readFile } from "node:fs/promises";

import { TokenBucket } from "./bucket.js";

interface LimitsFile {
  ratePerMinute: number;
  burst: number;
}

export class Limits {
  constructor(
    readonly ratePerMinute: number,
    readonly burst: number,
  ) {}

  newBucket(now: number): TokenBucket {
    return new TokenBucket(this.burst, this.ratePerMinute / 60, this.burst, now);
  }

  // Seconds until an untouched bucket is full again and can be dropped.
  idleTtl(): number {
    return Math.ceil((this.burst * 60) / this.ratePerMinute) + 1;
  }

  static from(raw: LimitsFile): Limits {
    return new Limits(raw.ratePerMinute, raw.burst);
  }
}

export class LimitsConfig {
  constructor(
    readonly defaults: Limits,
    readonly overrides: ReadonlyMap<string, Limits> = new Map(),
  ) {}

  forKey(apiKey: string): Limits {
    return this.overrides.get(apiKey) ?? this.defaults;
  }
}

export async function loadConfig(path = "config.json"): Promise<LimitsConfig> {
  const raw = JSON.parse(await readFile(path, "utf8")) as {
    default: LimitsFile;
    overrides?: Record<string, LimitsFile>;
  };
  const overrides = new Map(
    Object.entries(raw.overrides ?? {}).map(([key, value]) => [key, Limits.from(value)]),
  );
  return new LimitsConfig(Limits.from(raw.default), overrides);
}
