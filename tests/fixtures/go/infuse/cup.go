package infuse

type Sipper interface {
	Sip() int
}

type Drinker interface {
	Sipper
	Gulp() int
}

type Cup struct {
	Level int
}

func (c Cup) Sip() int { return c.Level }

func (c *Cup) Refill() *Cup { return c }

func (c Cup) WriteString(s string) int { return len(s) }

func (c Cup) Len() int { return c.Level }

func (c Cup) Close() error { return nil }

func (c Cup) Weekday() int { return c.Level }

type Mug struct {
	Cup
	Handle int
}

func (m Mug) Sip() int { return 2 }

func (m Mug) Warm() Stove { return Stove{} }

type Thermos struct {
	Mug
}

type Tray struct {
	Sipper
}

type Stove struct {
	Flame int
}

func (s Stove) Sip() int { return 3 }

func (s Stove) Gulp() int { return 4 }

type Blend struct {
	leaf  string
	water int
}

func NewBlend() *Blend { return &Blend{} }

func (b *Blend) Leaf(name string) *Blend { return b }

func (b *Blend) Water(ml int) *Blend { return b }

func (b *Blend) Build() Mug { return Mug{} }

func PourMug() Mug { return Mug{} }

func StoveAt(n int) *Stove { return &Stove{} }

func Pair() (Mug, Stove) { return Mug{}, Stove{} }

func Brew() int { return 5 }

func First[T Sipper](xs []T) T { return xs[0] }

func Last[T Sipper](xs ...T) *T { return &xs[len(xs)-1] }

func ByName[K comparable, V Sipper](m map[K]V, k K) V { return m[k] }

func Echo[Mug Sipper](m Mug) Mug { return m }
