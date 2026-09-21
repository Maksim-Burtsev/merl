//go:build !windows

package platform

func (t *Timer) Tick() int {
	return 2
}
