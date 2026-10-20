package ratelimit

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/redis/go-redis/v9"
)

// BucketStore keeps buckets between requests. A Get and the Set after it are not
// atomic, so two requests at once may both spend the same token.
type BucketStore interface {
	// Get returns nil and no error when the key has no bucket.
	Get(ctx context.Context, key string) (*TokenBucket, error)
	Set(ctx context.Context, key string, bucket *TokenBucket, ttl time.Duration) error
}

// MemoryStore keeps buckets in a map with Redis-like expiry. For tests and local runs.
type MemoryStore struct {
	mu    sync.Mutex
	items map[string]memoryItem
}

type memoryItem struct {
	bucket    TokenBucket
	expiresAt time.Time
}

func NewMemoryStore() *MemoryStore {
	return &MemoryStore{items: make(map[string]memoryItem)}
}

func (s *MemoryStore) Get(_ context.Context, key string) (*TokenBucket, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	item, ok := s.items[key]
	if !ok || !time.Now().Before(item.expiresAt) {
		delete(s.items, key)
		return nil, nil
	}
	bucket := item.bucket
	return &bucket, nil
}

func (s *MemoryStore) Set(_ context.Context, key string, bucket *TokenBucket, ttl time.Duration) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.items[key] = memoryItem{bucket: *bucket, expiresAt: time.Now().Add(ttl)}
	return nil
}

type RedisStore struct {
	Client *redis.Client
	Prefix string
}

func (s RedisStore) Get(ctx context.Context, key string) (*TokenBucket, error) {
	fields, err := s.Client.HGetAll(ctx, s.Prefix+key).Result()
	if err != nil {
		return nil, fmt.Errorf("get bucket %s: %w", key, err)
	}
	if len(fields) == 0 {
		return nil, nil
	}
	return bucketFromFields(fields)
}

func (s RedisStore) Set(ctx context.Context, key string, bucket *TokenBucket, ttl time.Duration) error {
	name := s.Prefix + key
	_, err := s.Client.TxPipelined(ctx, func(pipe redis.Pipeliner) error {
		pipe.HSet(ctx, name, bucket.fields())
		pipe.Expire(ctx, name, ttl)
		return nil
	})
	if err != nil {
		return fmt.Errorf("set bucket %s: %w", key, err)
	}
	return nil
}
