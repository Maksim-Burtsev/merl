//go:build !fast

package platform

type Codec struct{}

func (c *Codec) Encode() int {
	return 2
}
