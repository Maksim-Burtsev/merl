package rival

type Damper struct {
	Draft int
}

func (d Damper) Open() int { return d.Draft }

type Flue struct{ Soot int }

func Grouped() int {
	type (
		Damper struct{ Vent int }
		Flue   int
	)
	var d Damper
	//    ^ d: rival/damper.go:13
	var f Flue = 1
	//    ^ d: rival/damper.go:14
	return d.Vent + int(f)
	//       ^ d: none; want rival/damper.go:13 (#1470)
}

func Own() int {
	type Damper struct{ Vent int }
	//   ^ d: rival/damper.go:3; want rival/damper.go:25 (#1470)
	d := Damper{Vent: 1}
	//          ^ d: none; want rival/damper.go:25 (#1470)
	return d.Vent
	//       ^ d: none; want rival/damper.go:25 (#1470)
}

func Shared() int {
	type Damper struct{ Draft int }
	d := Damper{Draft: 2}
	//          ^ d: rival/damper.go:4; want rival/damper.go:34 (#1470)
	return d.Draft
	//       ^ d: rival/damper.go:4; want rival/damper.go:34 (#1470)
}

func Wrapped() int {
	type Damper struct {
		Draft int
	}
	d := Damper{Draft: 3}
	//          ^ d: rival/damper.go:4; want rival/damper.go:43 (#1470)
	return d.Draft
	//       ^ d: rival/damper.go:4; want rival/damper.go:43 (#1470)
}

func Field() int {
	rows := []struct{ Flue Flue }{}
	//                ^ d: rival/damper.go:9; want none (#1470)
	//                     ^ d: rival/damper.go:9
	return len(rows)
}

func Nested() int {
	type (
		Grate struct {
			Damper int
		}
	)
	var d Damper
	//    ^ d: rival/damper.go:3
	return d.Draft + Grate{}.Damper
}
