package infuse

import (
	"bytes"
	"os"
	"strings"
	"time"

	heat "example.com/fixture/infuse/burner"
	. "example.com/fixture/infuse/burner/spark"
)

const menu = `
func (m Mug) Sip() int {
type Stove struct {
`

/*
func Brew() int {
type Cup struct {
*/

type Order struct {
	Cup  Cup    `json:"func Sip() int {"`
	Note string `json:"type Mug struct {"`
}

func Outside() int {
	var sb strings.Builder
	sb.WriteString("green")
	// ^ d: none
	//  status: via sb: strings.Builder
	buf := new(bytes.Buffer)
	n := buf.Len()
	//       ^ d: none
	f, err := os.Open("leaves")
	if err == nil {
		_ = f.Close()
		//    ^ d: picker fields.go:25, infuse/cup.go:24, store/store.go:15; want none (#1204)
	}
	now := time.Now()
	return n + int(now.Weekday()) + strings.NewReader("x").Len()
	//                 ^ d: infuse/cup.go:26; want none (#1204)
	//                                                     ^ d: infuse/cup.go:22; want none (#1204)
}

func Chain() int {
	mug := NewBlend().
		Leaf("green").
		Water(80).
		Build()
//^ d: infuse/cup.go:64
	return mug.Sip() + NewBlend().Leaf("black").Build().Sip() + PourMug().Sip() + StoveAt(1).Sip()
	//         ^ d: infuse/cup.go:33
	//                            ^ d: infuse/cup.go:60
	//                                                  ^ d: infuse/cup.go:33
	//                                                                    ^ d: infuse/cup.go:33
	//                                                                                       ^ d: infuse/cup.go:49
}

func Refills(cup *Cup) int {
	cup = cup.Refill()
	//        ^ d: infuse/cup.go:18
	return cup.Refill().Refill().Sip()
	//                           ^ d: infuse/cup.go:16
}

func Shadows(cup Mug, stoves []Stove) int {
	total := 0
	{
		cup := cup.Warm()
		//         ^ d: infuse/cup.go:35
		total += cup.Sip()
		//           ^ d: infuse/cup.go:49
	}
	for _, cup := range stoves {
		total += cup.Sip()
		//           ^ d: infuse/cup.go:49
	}
	for i, cup := 0, (Stove{}); i < 1; i++ {
		total += cup.Sip()
		//           ^ d: picker infuse/burner/burner.go:7, infuse/cup.go:16, infuse/cup.go:33, infuse/cup.go:49; want infuse/cup.go:49 (#1202)
	}
	mug, stove := Pair()
	total += mug.Sip() + stove.Gulp()
	//           ^ d: infuse/cup.go:33
	//                         ^ d: infuse/cup.go:51
	brew := Brew()
	//      ^ d: infuse/cup.go:72
	return total + brew + cup.Sip()
	//             ^ d: infuse/tasting.go:88
	//                        ^ d: infuse/cup.go:33
}

func Switches(v any) int {
	switch c := v.(type) {
	case Mug:
		return c.Sip()
		//       ^ d: infuse/cup.go:33
	case Stove:
		return c.Sip()
		//       ^ d: infuse/cup.go:49
	}
	if m, ok := v.(Mug); ok {
		return m.Handle
		//       ^ d: infuse/cup.go:30
	}
	return 0
}

func Embeds(th Thermos, tr Tray, d Drinker) int {
	return th.Sip() + th.Refill().Level + th.Level + tr.Sip() + d.Sip() + d.Gulp()
	//        ^ d: infuse/cup.go:33
	//                   ^ d: infuse/cup.go:18
	//                                       ^ d: infuse/cup.go:13
	//                                                  ^ d: infuse/cup.go:4
	//                                                            ^ d: infuse/cup.go:4
	//                                                                      ^ d: infuse/cup.go:9
}

func Imports() int {
	b := heat.Light()
	//        ^ d: infuse/burner/burner.go:9
	c := Strike()
	//   ^ d: infuse/burner/spark/spark.go:7
	var k heat.Cup
	//         ^ d: infuse/burner/burner.go:11
	return b.Sip() + c.Glow + k.Level
	//       ^ d: infuse/burner/burner.go:7
	//                 ^ d: infuse/burner/spark/spark.go:4
	//                          ^ d: infuse/burner/burner.go:12
}

func Generics(mugs []Mug) int {
	return First(mugs).Sip() + Drink(Stove{})
	//                 ^ d: picker infuse/burner/burner.go:7, infuse/cup.go:16, infuse/cup.go:33, infuse/cup.go:49; want infuse/cup.go:33 (#1203)
}

func Drink[T Sipper](t T) int {
	return t.Sip()
	//       ^ d: picker infuse/burner/burner.go:7, infuse/cup.go:16, infuse/cup.go:33, infuse/cup.go:49; want infuse/cup.go:4 (#1203)
}

func Named() (cup Stove) {
	_ = cup.Sip()
	//      ^ d: infuse/cup.go:49
	return
}

func Strings(o Order) int {
	return o.Cup.Sip() + len(menu)
	//       ^ d: infuse/tasting.go:24
	//           ^ d: infuse/cup.go:16
}

func OneLine() int {
	mug := NewBlend().Leaf("green").Build()
	return mug.Sip()
	//         ^ d: infuse/cup.go:33
}
