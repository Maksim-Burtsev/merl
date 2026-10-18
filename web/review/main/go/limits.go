package ratelimit

import (
	"fmt"
	"math"
	"time"

	"github.com/BurntSushi/toml"
)

type Limits struct {
	RatePerMinute int `toml:"rate_per_minute"`
	Burst         int `toml:"burst"`
}

func (l Limits) NewBucket(now time.Time) *TokenBucket {
	return &TokenBucket{
		Capacity:        float64(l.Burst),
		RefillPerSecond: float64(l.RatePerMinute) / 60,
		Tokens:          float64(l.Burst),
		UpdatedAt:       now,
	}
}

// IdleTTL is how long an untouched bucket takes to fill up again; after that
// the store may drop it.
func (l Limits) IdleTTL() time.Duration {
	seconds := math.Ceil(float64(l.Burst) * 60 / float64(l.RatePerMinute))
	return time.Duration(seconds+1) * time.Second
}

type LimitsConfig struct {
	Default   Limits            `toml:"default"`
	Overrides map[string]Limits `toml:"overrides"`
}

func (c LimitsConfig) ForKey(apiKey string) Limits {
	if l, ok := c.Overrides[apiKey]; ok {
		return l
	}
	return c.Default
}

func LoadConfig(path string) (LimitsConfig, error) {
	var c LimitsConfig
	if _, err := toml.DecodeFile(path, &c); err != nil {
		return LimitsConfig{}, fmt.Errorf("load limits from %s: %w", path, err)
	}
	if c.Default.RatePerMinute <= 0 {
		return LimitsConfig{}, fmt.Errorf("load limits from %s: default rate_per_minute must be positive", path)
	}
	return c, nil
}
