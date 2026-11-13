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
	//       ^ d: rival/damper.go:13
}

func Own() int {
	type Damper struct{ Vent int }
	//   ^ d: rival/damper.go:25
	d := Damper{Vent: 1}
	//          ^ d: rival/damper.go:25
	return d.Vent
	//       ^ d: rival/damper.go:25
}

func Shared() int {
	type Damper struct{ Draft int }
	d := Damper{Draft: 2}
	//          ^ d: rival/damper.go:34
	return d.Draft
	//       ^ d: rival/damper.go:34
}

func Wrapped() int {
	type Damper struct {
		Draft int
	}
	d := Damper{Draft: 3}
	//          ^ d: rival/damper.go:43
	return d.Draft
	//       ^ d: rival/damper.go:43
}

func Field() int {
	rows := []struct{ Flue Flue }{}
	//                ^ d: none
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

func Block(hot bool) int {
	if hot {
		type Damper int
		//   ^ d: rival/damper.go:71
		return int(Damper(1))
	}
	return 0
}

func After(hot bool) int {
	if hot {
		type Damper struct{ Draft int }
		_ = Damper{Draft: 1}
		//         ^ d: rival/damper.go:80
	}
	d := Damper{Draft: 2}
	//          ^ d: rival/damper.go:4
	return d.Draft
	//       ^ d: rival/damper.go:4
}

func Defined() int {
	type Opts Damper
	o := Opts{Draft: 1}
	//        ^ d: rival/damper.go:4
	return o.Draft
}

func Aliased() int {
	type Opts = Damper
	o := Opts{Draft: 1}
	//        ^ d: rival/damper.go:4
	return o.Draft
}

func Fields() int {
	rows := []struct {
		Draft int
	}{}
	pairs := []struct{ Soot int; Flue, Damper Flue }{}
	//                           ^ d: none
	//                                 ^ d: none
	//                                        ^ d: rival/damper.go:9
	held := struct{ Flue }{}
	//              ^ d: rival/damper.go:9
	return len(rows) + len(pairs) + held.Soot
}
