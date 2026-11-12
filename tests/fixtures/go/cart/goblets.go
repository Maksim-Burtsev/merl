package cart

type Goblet struct{}

func (g Goblet) Quaff() {}

type Rack[Goblet any] struct {
	held Goblet
	//   ^ d: cart/goblets.go:7
}

func Fill[Goblet any](x Goblet) Goblet {
//                      ^ d: cart/goblets.go:12
	x.Quaff()
	//^ d: none
	return x
}

func (r Rack[Goblet]) Get(x Goblet) {}
//                          ^ d: cart/goblets.go:19

func Plain(x Goblet) { x.Quaff() }
//           ^ d: cart/goblets.go:3
//                       ^ d: cart/goblets.go:5

type Row [N]Goblet
//          ^ d: cart/goblets.go:3

const N = 2

type Toaster interface {
	Clink()
}

func (g Goblet) Clink() {}

func Bounded[T Toaster](x T) {
	x.Clink()
	//^ d: cart/goblets.go:32
}

func Unbound[T any](x T) {
	x.Quaff()
	//^ d: none
}

func (r Rack[Goblet]) Use(x Goblet, y *Goblet) {
	x.Quaff()
	//^ d: none
	y.Quaff()
	//^ d: none
	r.held.Quaff()
	//     ^ d: none
}

func Tilde[Goblet ~int](x Goblet) {
	var y Goblet
	y.Quaff()
	//^ d: none
}

func Wrapped[Goblet any](
	x Goblet,
) {
	x.Quaff()
	//^ d: none
}

type Shelf[T Toaster] struct {
	held T
}

func (s Shelf[T]) Ring(x T) {
	x.Clink()
	//^ d: cart/goblets.go:32
	s.held.Clink()
	//     ^ d: cart/goblets.go:32
}

func (s *Shelf[U]) Again(x U) {
	x.Clink()
	//^ d: cart/goblets.go:32
}

type Pourer interface{ Pour() }

func (g Goblet) Pour() {}

type Jug[T Pourer] struct{}

func (j Jug[T]) Tip(x T) {
	x.Pour()
	//^ d: cart/goblets.go:85
}

func Tilt[T Pourer](x T) {
	x.Pour()
	//^ d: cart/goblets.go:85
}

type Loose[T any] struct{}

func (l Loose[T]) Drop(x T) {
	x.Pour()
	//^ d: none
}

type Cup[T any] struct {
	held T
}

func Sip[T Toaster](c Cup[T]) {
	c.held.Clink()
	//     ^ d: cart/goblets.go:32
}

func Inline[T interface{ Clink() }](x T) {
	x.Clink()
	//^ d: cart/goblets.go:117
}

type Spiller interface{ Tip(); Pour() }

func Spill(s Spiller) {
	s.Pour()
	//^ d: cart/goblets.go:122
}

func Ladle[T Spiller](x T) {
	x.Pour()
	//^ d: cart/goblets.go:122
}

func Swap[U Toaster, T any](c Cup[U]) {
	c.held.Clink()
	//     ^ d: cart/goblets.go:32
}

func Spare[U any](c Cup[U]) {
	c.held.Clink()
	//     ^ d: none
}

func Concrete(c Cup[Goblet]) {
	c.held.Clink()
	//     ^ d: cart/goblets.go:35
}

func Pointed[T Toaster](c *Cup[T]) {
	c.held.Clink()
	//     ^ d: cart/goblets.go:32
}

type Pair[A any, B any] struct {
	first  A
	second B
}

func Pick[T Toaster](p Pair[Goblet, T]) {
	p.second.Clink()
	//       ^ d: cart/goblets.go:32
	p.first.Clink()
	//      ^ d: cart/goblets.go:35
}

func Held[T interface{ Clink() }](c Cup[T]) {
	c.held.Clink()
	//     ^ d: cart/goblets.go:166
}

func Shadow[T interface{ Clink() }](x T) {
	{
		x := Goblet{}
		x.Clink()
		//^ d: cart/goblets.go:35
	}
}
