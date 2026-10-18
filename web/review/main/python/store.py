import time
from typing import Protocol

from redis.asyncio import Redis

from bucket import TokenBucket


class BucketStore(Protocol):
    """Where buckets live between requests. A get and the set after it are not atomic,
    so two requests at once may both spend the same token."""

    async def get(self, key: str) -> TokenBucket | None: ...

    async def set(self, key: str, bucket: TokenBucket, ttl: int) -> None: ...


class MemoryStore:
    """Keeps buckets in a dict with Redis-like expiry. For tests and local runs."""

    def __init__(self) -> None:
        self._items: dict[str, tuple[dict[str, str], float]] = {}

    async def get(self, key: str) -> TokenBucket | None:
        item = self._items.get(key)
        if item is None:
            return None
        fields, expires_at = item
        if expires_at <= time.monotonic():
            del self._items[key]
            return None
        return TokenBucket.from_fields(fields)

    async def set(self, key: str, bucket: TokenBucket, ttl: int) -> None:
        self._items[key] = (bucket.to_fields(), time.monotonic() + ttl)


class RedisStore:
    def __init__(self, redis: Redis, prefix: str = "ratelimit:") -> None:
        self._redis = redis
        self._prefix = prefix

    async def get(self, key: str) -> TokenBucket | None:
        fields = await self._redis.hgetall(self._prefix + key)
        if not fields:
            return None
        return TokenBucket.from_fields({k.decode(): v.decode() for k, v in fields.items()})

    async def set(self, key: str, bucket: TokenBucket, ttl: int) -> None:
        name = self._prefix + key
        async with self._redis.pipeline(transaction=True) as pipe:
            pipe.hset(name, mapping=bucket.to_fields())
            pipe.expire(name, ttl)
            await pipe.execute()
