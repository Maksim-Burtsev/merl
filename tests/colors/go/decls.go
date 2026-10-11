// Package shapes holds the declarations a Go file is made of.
package shapes

import (
	"errors"
	"fmt"
	stdio "io"
	. "math"
	_ "net/http/pprof"
)

import "strings"

// Kind says what a shape is.
type Kind int

const (
	Circle Kind = iota
	Square
	maxKinds = 1 << 4
)

const Pi2 = 2 * Pi

var (
	ErrEmpty      = errors.New("empty")
	defaultName   = "shape"
	count       int64
	ratio         float32 = 0.5e-3
)

// Shape is drawn on a canvas.
type Shape interface {
	Area() float64
	fmt.Stringer
	~int | ~float64
}

type Point struct {
	X, Y int `json:"x" yaml:"y"`
	label    *string
	stdio.Reader
	tags     map[string][]byte
	next     chan<- Point
	done     <-chan struct{}
	fn       func(int, ...string) (bool, error)
}

type Pair[K comparable, V any] struct {
	Key   K
	Value V
}

type Alias = Pair[string, int]

func (p *Point) Area() float64 {
	return float64(p.X * p.Y)
}

func (Point) String() string { return strings.ToUpper(defaultName) }

func Map[T, U any](xs []T, f func(T) U) []U {
	out := make([]U, 0, len(xs))
	for _, x := range xs {
		out = append(out, f(x))
	}
	return out
}
