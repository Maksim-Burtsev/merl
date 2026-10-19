package rival

type (
	// Leaf is one parsed unit.
	Leaf struct {
		Vein int
		Stem string
	}
)

type Branch struct {
	leaves []Leaf
}

func (b Branch) Vein() int { return len(b.leaves) }

func Prune() int {
	cases := []struct {
		input  []byte
		leaves []Leaf
	}{
		{
			input: []byte(`a:1`),
			leaves: []Leaf{
				{Vein: 1, Stem: `a`},
				//^ d: rival/table.go:6
				//        ^ d: rival/table.go:7
			},
		},
	}
	return len(cases)
}

func Graft(l Leaf) int {
	return l.Vein + len(l.Stem)
	//     ^ d: rival/table.go:34
	//       ^ d: rival/table.go:6
	//                    ^ d: rival/table.go:7
}

var _ = Leaf{}
//      ^ d: rival/table.go:5

type (
	Sprig = Leaf
	Twig  Leaf
)

func Bud(s Sprig) []any {
	return []any{s.Vein, Sprig{Stem: "s"}, Twig{Vein: 2}}
	//             ^ d: rival/table.go:6
	//                         ^ d: rival/table.go:7
	//                                          ^ d: rival/table.go:6
}

type Bough []*Bundle

type Bundle struct {
	*Leaf
	Knot int
}

var _ = Bough{
	{Leaf: nil, Knot: 1},
	//^ d: rival/table.go:59
	//          ^ d: rival/table.go:60
}
