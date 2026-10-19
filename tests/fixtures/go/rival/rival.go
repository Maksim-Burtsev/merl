package rival

import (
	"strings"

	"example.com/fixture/rival/twin"
)

const recipe = `
func Brew() int {
type Teapot struct {
func (k Kettle) Pour() int {
`

/*
func Mug() int {
type Kettle struct {
*/

type Kettle struct {
	Spout int
	Ring  func() int
}

func (k Kettle) Pour() int { return k.Spout }

type Teapot struct {
	Spout string
}

func (t Teapot) Pour() int { return len(t.Spout) }

func (t Teapot) Mug() int { return 1 }

func Mug() int { return 4 }

type Pourer interface {
	Pour() int
}

type Samovar struct {
	Kettle
}

func (s Samovar) Pour() int { return 5 }

type Urn struct {
	Kettle
}

type Tin[T any] struct {
	leaf T
}

func (t Tin[T]) Lid() T { return t.leaf }

func Brew() int {
	k, err := Kettle{}, error(nil)
	t, err := Teapot{}, err
	_ = err
	//  ^ d: rival/rival.go:58
	return k.Pour() + t.Pour()
	//       ^ d: rival/rival.go:25
	//                  ^ d: rival/rival.go:31
}

func Steep() int {
	k := Kettle{}
	pour := k.Pour
	//        ^ d: rival/rival.go:25
	lift := Teapot.Pour
	//             ^ d: rival/rival.go:31
	return pour() + lift(Teapot{})
	//     ^ d: rival/rival.go:69
	//              ^ d: rival/rival.go:71
}

func Infuse(twin string) int {
	return len(twin)
	//         ^ d: rival/rival.go:78
}

func Strain() int {
	strings := []string{"a"}
	return len(strings)
	//         ^ d: rival/rival.go:84
}

func Sieve() string {
	return strings.ToUpper("a")
	//     ^ d: rival/rival.go:4
}

func Whistle() int {
	return twin.Whistle() + Mug() + twin.Mug()
	//          ^ d: rival/twin/twin.go:9
	//                      ^ d: rival/rival.go:35
	//                                   ^ d: rival/twin/twin.go:11
}

func Recipe() int {
	return Brew() + len(recipe)
	//     ^ d: rival/rival.go:57
	//                  ^ d: rival/rival.go:9
}

func Pick() int {
	near := Kettle{}
	far := twin.Kettle{}
	//          ^ d: rival/twin/twin.go:3
	return near.Pour() + far.Pour()
	//          ^ d: rival/rival.go:25
	//                       ^ d: rival/twin/twin.go:7
}

func (k Kettle) Fill() int {
	Spout := 3
	return Spout + k.Spout + k.Ring()
	//     ^ d: rival/rival.go:117
	//               ^ d: rival/rival.go:21
	//                         ^ d: rival/rival.go:22
}

func Pack() Kettle {
	Spout := 1
	return Kettle{Spout: Spout}
	//            ^ d: rival/rival.go:21
	//                   ^ d: rival/rival.go:125
}

func Simmer() int {
	k := Kettle{}
	if k := (Teapot{}); k.Spout != "" {
		return k.Pour()
		//       ^ d: rival/rival.go:31
	}
	return k.Pour()
	//       ^ d: rival/rival.go:25
}

func Ladle() int {
	cup := Kettle{}
	f := func() int {
		cup := Teapot{}
		return cup.Pour()
		//         ^ d: rival/rival.go:31
	}
	return f() + cup.Pour()
	//               ^ d: rival/rival.go:25
}

func Serves(p Pourer, s Samovar, u Urn) int {
	return p.Pour() + s.Pour() + s.Kettle.Pour() + u.Pour()
	//       ^ d: rival/rival.go:38
	//                  ^ d: rival/rival.go:45
	//                                    ^ d: rival/rival.go:25
	//                                               ^ d: rival/rival.go:25
}

func Caddy() string {
	return Tin[string]{}.Lid()
	//                   ^ d: rival/rival.go:55
}

func (t Teapot) Blend() int {
	return t.Mug() + Mug()
	//       ^ d: rival/rival.go:33
	//               ^ d: rival/rival.go:35
}

func Cosy() int {
	near := Kettle{}
	//      ^ d: rival/rival.go:20
	return Teapot{}.Pour() + near.Pour()
	//     ^ d: rival/rival.go:27
	//                            ^ d: rival/rival.go:25
}

type Kettles []Kettle

func (ks Kettles) Pour() int { return len(ks) }

func Rack(ks []Kettle, ts []*Teapot, byName map[string]Kettle, pair [2]Teapot) int {
	i := 0
	return ks[i].Pour() + ts[i+1].Pour() + byName["a"].Spout + pair[0].Pour()
	//           ^ d: rival/rival.go:25
	//                            ^ d: rival/rival.go:31
	//                                                 ^ d: rival/rival.go:21
	//                                                                 ^ d: rival/rival.go:31
}

func Shelf(cups []any, all Kettles, ks []Kettle) int {
	return cups[0].(Kettle).Pour() + all[1:].Pour() + all[0].Pour() + ks[1:2][0].Pour()
	//                      ^ d: !jump
	//                                       ^ d: !jump
	//                                                       ^ d: rival/rival.go:25
	//                                                                           ^ d: !jump
}

type Pantry struct {
	shelf []*Teapot
	cups  []any
}

func (p Pantry) Stock() int {
	return p.shelf[0].Pour() + p.cups[0].Pour()
	//                ^ d: rival/rival.go:31
	//                                   ^ d: !jump
}

func Pots(pots ...*Teapot) int {
	return pots[0].Pour()
	//             ^ d: rival/rival.go:31
}

func Cups(cups ...Kettle) int {
	for _, cup := range cups {
		_ = cup.Pour()
		//      ^ d: rival/rival.go:25
	}
	return cups.Pour()
	//          ^ d: !jump
}
