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
	//^ d: cart/goblets.go:5; want none (#1438)
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
