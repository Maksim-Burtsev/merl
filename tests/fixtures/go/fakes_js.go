//go:build js

package main

import "example.com/fixture/fakes"

// Under this tag `fakes` is the import; under the other one, the package's variable.
func FakesWipe() {
	s := fakes.NewStub()
	s.Wipe()
	//^ d: fakes/fakes.go:13
	//  status: via fakes.NewStub() *Stub
}
