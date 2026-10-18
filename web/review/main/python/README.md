# ratelimit

Rate limiting for the public API. Every request takes one token from the bucket of its
API key; an empty bucket answers 429 with a `Retry-After` header.

Limits live in `limits.toml`. Buckets live in Redis in production and in memory in tests.
