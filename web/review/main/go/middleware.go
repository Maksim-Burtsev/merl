package ratelimit

import (
	"log/slog"
	"math"
	"net/http"
	"strconv"
	"time"
)

// Middleware spends one token per request from the bucket of the caller's API key
// and answers 429 once the bucket is empty.
func Middleware(store BucketStore, config LimitsConfig, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		apiKey := r.Header.Get("X-Api-Key")
		if apiKey == "" {
			http.Error(w, "missing API key", http.StatusUnauthorized)
			return
		}

		ctx := r.Context()
		limits := config.ForKey(apiKey)
		now := time.Now()
		bucket, err := store.Get(ctx, apiKey)
		if err != nil {
			slog.ErrorContext(ctx, "load bucket", "err", err)
			http.Error(w, "rate limiter unavailable", http.StatusServiceUnavailable)
			return
		}
		if bucket == nil {
			bucket = limits.NewBucket(now)
		}
		allowed := bucket.Take(now)
		if err := store.Set(ctx, apiKey, bucket, limits.IdleTTL()); err != nil {
			slog.ErrorContext(ctx, "save bucket", "err", err)
		}

		if !allowed {
			wait := bucket.RetryAfter(now)
			w.Header().Set("Retry-After", strconv.Itoa(int(math.Ceil(wait.Seconds()))))
			http.Error(w, "rate limit exceeded", http.StatusTooManyRequests)
			return
		}
		next.ServeHTTP(w, r)
	})
}
