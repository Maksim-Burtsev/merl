use std::time::Duration;

use ratelimit::{BucketStore, Limits, MemoryStore};

const LIMITS: Limits = Limits { rate_per_minute: 60, burst: 3 };

#[test]
fn take_spends_the_tokens_then_refuses() {
    let mut bucket = LIMITS.new_bucket(0.0);
    let taken: Vec<bool> = (0..4).map(|_| bucket.take(0.0)).collect();
    assert_eq!(taken, [true, true, true, false]);
}

#[test]
fn take_refills_at_the_rate() {
    let mut bucket = LIMITS.new_bucket(0.0);
    for _ in 0..3 {
        bucket.take(0.0);
    }
    assert!(bucket.take(1.0));
    assert!(!bucket.take(1.0));
}

#[test]
fn retry_after_waits_for_one_token() {
    let mut bucket = LIMITS.new_bucket(0.0);
    for _ in 0..3 {
        bucket.take(0.0);
    }
    assert!((bucket.retry_after(0.25) - 0.75).abs() < 1e-9);
}

#[test]
fn retry_after_is_zero_with_tokens_left() {
    let mut bucket = LIMITS.new_bucket(0.0);
    assert_eq!(bucket.retry_after(0.0), 0.0);
}

#[tokio::test]
async fn memory_store_round_trips_a_bucket() {
    let store = MemoryStore::default();
    let mut bucket = LIMITS.new_bucket(10.0);
    bucket.take(10.0);
    store.set("key-1", &bucket, Duration::from_secs(60)).await.unwrap();
    assert_eq!(store.get("key-1").await.unwrap(), Some(bucket));
}
