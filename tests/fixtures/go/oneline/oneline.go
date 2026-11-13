package oneline

type Wick struct{ Glow int }

func (w *Wick) Trim() {}

type Snuff struct{ Glow int }

func (s *Snuff) Trim() {}

type Taper struct{ Wick }

type Votive struct{ *Wick }

type Tagged struct{ Wick `json:"wick"` }

type Shaded struct{ Wick; Glow string }

type Twin struct{ Wick; Snuff }

func (t *Taper) Light() {
	f := t.Trim
	//     ^ d: oneline/oneline.go:5
	t.Glow = 1
	//^ d: oneline/oneline.go:3
	_ = f
}

func (v *Votive) Light() {
	v.Trim()
	//^ d: oneline/oneline.go:5
}

func (g *Tagged) Light() {
	g.Glow = 2
	//^ d: oneline/oneline.go:3
}

func (s *Shaded) Light() {
	s.Glow = "x"
	//^ d: oneline/oneline.go:17
}

func (t *Twin) Light() {
	t.Wick.Trim()
	//     ^ d: oneline/oneline.go:5
	t.Snuff.Glow = 3
	//      ^ d: oneline/oneline.go:7
}
