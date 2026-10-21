package burner

type Burner struct {
	Flame int
}

func (b Burner) Sip() int { return 6 }

func Light() Burner { return Burner{} }

type Cup struct {
	Level int
}
