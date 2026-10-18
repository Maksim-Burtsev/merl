# ratelimit

Rate limiting for the public API, as `net/http` middleware. Every request takes one token
from the bucket of its API key; an empty bucket answers 429 with a `Retry-After` header.

Limits live in `config.toml`. Buckets live in Redis in production and in memory in tests.
