from bucket import TokenBucket
from plans import Limits
from store import MemoryStore

LIMITS = Limits(rate_per_minute=60, burst=3)


def test_take_spends_the_burst_then_refuses():
    bucket = LIMITS.new_bucket(now=0.0)
    assert [bucket.take(now=0.0) for _ in range(4)] == [True, True, True, False]


def test_bucket_refills_at_the_rate():
    bucket = LIMITS.new_bucket(now=0.0)
    for _ in range(3):
        bucket.take(now=0.0)
    assert bucket.take(now=1.0)
    assert not bucket.take(now=1.0)


def test_retry_after_waits_for_one_token():
    bucket = LIMITS.new_bucket(now=0.0)
    for _ in range(3):
        bucket.take(now=0.0)
    assert bucket.retry_after(now=0.25) == 1


def test_retry_after_rounds_a_short_wait_up_to_a_second():
    bucket = LIMITS.new_bucket(now=0.0)
    for _ in range(3):
        bucket.take(now=0.0)
    assert bucket.retry_after(now=0.999) == 1


def test_retry_after_is_zero_with_tokens_left():
    bucket = LIMITS.new_bucket(now=0.0)
    assert bucket.retry_after(now=0.0) == 0.0


def test_burst_follows_the_plan():
    bucket = TokenBucket.for_plan(Limits(rate_per_minute=600, burst=2), now=0.0)
    assert sum(bucket.take(now=0.0) for _ in range(100)) == 60


async def test_memory_store_round_trips_a_bucket():
    store = MemoryStore()
    bucket = LIMITS.new_bucket(now=10.0)
    bucket.take(now=10.0)
    await store.set("key-1", bucket, ttl=60)
    assert await store.get("key-1") == bucket
