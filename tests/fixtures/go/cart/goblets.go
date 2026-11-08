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
