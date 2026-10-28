package shop

type Hello struct{}

func (h *Hello) Context() int { return 0 }

func NewContext() (int, func()) { return 0, func() {} }

type Policy struct{}

type Config struct {
	Greet func(h *Hello) int
}

// A local of a function literal in a composite literal is no package-level ctx (#1269).
func (p *Policy) build() *Config {
	cfg := &Config{
		Greet: func(h *Hello) int {
			ctx := h.Context()
			return ctx
			//     ^ d: shop/handshake.go:19
		},
	}
	return cfg
}

func provision(names []string) {
	for range names {
		ctx, _ := NewContext()
		//^ d: none
		_ = ctx
	}
}
