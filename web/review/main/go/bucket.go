package ratelimit

import (
	"fmt"
	"strconv"
	"time"
)

// TokenBucket is the token bucket of one API key.
type TokenBucket struct {
	Capacity        float64
	RefillPerSecond float64
	Tokens          float64
	UpdatedAt       time.Time
}

func (b *TokenBucket) Take(now time.Time) bool {
	b.refill(now)
	if b.Tokens < 1 {
		return false
	}
	b.Tokens--
	return true
}

func (b *TokenBucket) RetryAfter(now time.Time) time.Duration {
	b.refill(now)
	missing := 1 - b.Tokens
	if missing <= 0 {
		return 0
	}
	return time.Duration(missing / b.RefillPerSecond * float64(time.Second))
}

func (b *TokenBucket) refill(now time.Time) {
	elapsed := max(0, now.Sub(b.UpdatedAt).Seconds())
	b.Tokens = min(b.Capacity, b.Tokens+elapsed*b.RefillPerSecond)
	b.UpdatedAt = now
}

func (b *TokenBucket) fields() map[string]any {
	return map[string]any{
		"capacity":          b.Capacity,
		"refill_per_second": b.RefillPerSecond,
		"tokens":            b.Tokens,
		"updated_at":        b.UpdatedAt.UnixNano(),
	}
}

func bucketFromFields(fields map[string]string) (*TokenBucket, error) {
	var b TokenBucket
	var err error
	parse := func(name string) float64 {
		v, perr := strconv.ParseFloat(fields[name], 64)
		if perr != nil && err == nil {
			err = fmt.Errorf("field %s: %w", name, perr)
		}
		return v
	}
	b.Capacity = parse("capacity")
	b.RefillPerSecond = parse("refill_per_second")
	b.Tokens = parse("tokens")
	b.UpdatedAt = time.Unix(0, int64(parse("updated_at")))
	return &b, err
}
