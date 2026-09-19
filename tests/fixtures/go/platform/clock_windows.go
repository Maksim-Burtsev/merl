package platform

// Clock is declared once per platform (#100): this file by its name, `clock_other.go` by its
// `//go:build` line.
type Clock struct{}

func NewClock() *Clock {
	return &Clock{}
}

func (c *Clock) Now() int {
	return 1
}

// From a file the host does not build, neither `Clock` is preferred.
func (c *Clock) Twice() int {
	return c.Now() * 2
}
