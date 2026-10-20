package ratelimit

import (
	"context"
	"testing"
	"time"
)

var t0 = time.Unix(1_700_000_000, 0)

func fullBucket() *TokenBucket {
	return &TokenBucket{Capacity: 3, RefillPerSecond: 1, Tokens: 3, UpdatedAt: t0}
}

func TestTakeSpendsTheTokensThenRefuses(t *testing.T) {
	b := fullBucket()
	for i, want := range []bool{true, true, true, false} {
		if got := b.Take(t0); got != want {
			t.Fatalf("take %d: got %v, want %v", i, got, want)
		}
	}
}

func TestTakeRefillsAtTheRate(t *testing.T) {
	b := fullBucket()
	for range 3 {
		b.Take(t0)
	}
	if !b.Take(t0.Add(time.Second)) {
		t.Fatal("no token one second later")
	}
}

func TestRetryAfterWaitsForOneToken(t *testing.T) {
	b := fullBucket()
	for range 3 {
		b.Take(t0)
	}
	if got := b.RetryAfter(t0.Add(250 * time.Millisecond)); got != time.Second {
		t.Fatalf("RetryAfter = %v, want 1s", got)
	}
}

func TestRetryAfterRoundsAShortWaitUpToASecond(t *testing.T) {
	b := fullBucket()
	for range 3 {
		b.Take(t0)
	}
	if got := b.RetryAfter(t0.Add(999 * time.Millisecond)); got != time.Second {
		t.Fatalf("RetryAfter = %v, want 1s", got)
	}
}

func TestBucketForPlanSizesTheBurst(t *testing.T) {
	b := BucketForPlan(Limits{RatePerMinute: 600}, t0)
	taken := 0
	for range 100 {
		if b.Take(t0) {
			taken++
		}
	}
	if taken != 60 {
		t.Fatalf("took %d tokens at once, want 60", taken)
	}
}

func TestMemoryStoreRoundTripsABucket(t *testing.T) {
	ctx := context.Background()
	store := NewMemoryStore()
	b := fullBucket()
	b.Take(t0)
	if err := store.Set(ctx, "key-1", b, time.Minute); err != nil {
		t.Fatal(err)
	}
	got, err := store.Get(ctx, "key-1")
	if err != nil || got == nil || got.Tokens != 2 {
		t.Fatalf("Get = %+v, %v", got, err)
	}
}
