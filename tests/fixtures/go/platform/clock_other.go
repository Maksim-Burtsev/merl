//go:build !windows

package platform

type Clock struct{}

func NewClock() *Clock {
	return &Clock{}
}

func (c *Clock) Now() int {
	return 2
}
