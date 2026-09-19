//go:build windows && !slow

package platform

// Gauge: a platform that has decided is not undone by a tag that is none.
type Gauge struct{}

func (g *Gauge) Read() int {
	return 1
}
